//! Command-line interface: query and download GitHub resources.
//!
//! Two subcommands keep the surface small:
//!
//! * `ls` — list the supported mirrors, or the assets of a release.
//! * `fetch` — download one asset (to `-o PATH`, or the file name in the
//!   current directory).

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::artifacts::GhArtifacts;
use crate::fetch::{self, Options};
use crate::{Proxy, Resource};

/// Query and download GitHub resources through mirrors.
#[derive(Debug, Parser)]
#[command(name = "github-proxy", version, about, long_about = None)]
struct Cli {
    /// Mirror to use.
    #[arg(short, long, value_enum, default_value_t = Proxy::Github)]
    proxy: Proxy,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List mirrors, or the assets of a release.
    ///
    /// With no repository, prints the supported mirrors. Otherwise prints one
    /// line per asset: human-readable size and file name.
    Ls {
        /// `owner/repo` or `owner/repo@tag` (the tag may contain `/`).
        repo: Option<String>,

        /// Keep only assets with exactly this name.
        #[arg(short, long, value_name = "NAME")]
        name: Option<String>,

        /// Print the download URLs instead of size and name.
        #[arg(short, long)]
        url: bool,
    },

    /// Download a resource.
    ///
    /// `SPEC` is a GitHub URL or `owner/repo[@tag][/name]`. Without a tag the
    /// latest release is used.
    Fetch {
        /// URL or `owner/repo[@tag][/name]`.
        spec: String,

        /// Asset name.
        ///
        /// Preferred form when the tag contains `/`, e.g.
        /// `owner/repo@release/1.0 a.zip`: the tag is then everything after
        /// `@`, with no guesswork. Also the only way to name an asset when the
        /// tag is omitted.
        name: Option<String>,

        /// Write to this file instead of the asset name in the current
        /// directory. An existing directory receives the asset name.
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
    },
}

/// CLI failure.
#[derive(Debug, Error)]
pub enum Error {
    /// The mirror cannot serve the requested kind of resource.
    #[error("{proxy} cannot serve {resource} (release assets need github, gh-proxy or xget)")]
    Unsupported {
        /// Selected mirror.
        proxy: Proxy,
        /// Requested resource.
        resource: Resource,
    },

    /// `SPEC` is neither a GitHub URL nor `owner/repo[@tag][/name]`.
    #[error(
        "invalid spec '{0}'\nexpected a GitHub URL or owner/repo[@tag][/name] (add the asset name)"
    )]
    InvalidSpec(String),

    /// The repository has no published release to resolve a tag from.
    #[error("{0} has no published release")]
    NoRelease(String),

    /// `--name` did not match any asset.
    #[error("no asset named '{name}' in {repo}\n{available}")]
    NoSuchAsset {
        /// Requested name.
        name: String,
        /// Release that was listed.
        repo: String,
        /// Known asset names, one per line.
        available: String,
    },

    /// The release could not be listed.
    #[error(transparent)]
    Fetch(#[from] fetch::Error),

    /// Writing the download failed.
    #[error("failed to write '{path}': {source}")]
    Write {
        /// Destination path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The async runtime could not start.
    #[error("failed to start the runtime: {0}")]
    Runtime(std::io::Error),
}

/// Parse arguments and run.
pub fn run() -> Result<(), Error> {
    let cli = Cli::parse();
    match cli.command {
        Command::Ls { repo, name, url } => match repo {
            Some(spec) => list_release(&spec, name.as_deref(), url, cli.proxy),
            None => {
                list_mirrors();
                Ok(())
            }
        },
        Command::Fetch { spec, name, output } => {
            fetch_resource(&spec, name.as_deref(), cli.proxy, output)
        }
    }
}

/// Print the supported mirrors and their capabilities.
fn list_mirrors() {
    for proxy in Proxy::ALL {
        let capability = if proxy.supports_release() {
            "files + releases"
        } else {
            "files only"
        };
        println!("{:<11} {capability}", proxy.as_str());
    }
}

/// Print one line per asset of `owner/repo[@tag]`: size and name, or URLs.
fn list_release(
    spec: &str,
    name: Option<&str>,
    urls_only: bool,
    proxy: Proxy,
) -> Result<(), Error> {
    let (owner, repo, tag) = parse_repo_spec(spec)?;
    let api = match &tag {
        Some(tag) => format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}"),
        None => format!("https://api.github.com/repos/{owner}/{repo}/releases/latest"),
    };

    let release: GhArtifacts = block_on(async {
        let token = crate::token::detect().await;
        fetch::json(
            &api,
            Options {
                token: token.as_deref(),
                timeout: None,
            },
        )
        .await
    })?;

    let mut assets: Vec<_> = release.assets.into_iter().collect();
    assets.sort_by(|a, b| a.name.cmp(&b.name));
    if assets.is_empty() {
        let suffix = tag.map(|tag| format!("@{tag}")).unwrap_or_default();
        eprintln!("no assets in {owner}/{repo}{suffix}");
        return Ok(());
    }

    if let Some(name) = name {
        assets.retain(|asset| asset.name == name);
        if assets.is_empty() {
            return Err(Error::NoSuchAsset {
                name: name.to_string(),
                repo: format!("{owner}/{repo}"),
                available: format!("run `github-proxy ls {spec}` to list them"),
            });
        }
    }

    // Sizes share one width so the columns line up.
    let width = assets
        .iter()
        .map(|asset| humansize::format_size(asset.size, humansize::BINARY).len())
        .max()
        .unwrap_or(0);

    for asset in assets {
        // The download URL carries the real tag, which is what `latest` needs.
        let Ok(resource) = asset.browser_download_url.parse::<Resource>() else {
            continue;
        };
        let Some(url) = resource.url(&proxy) else {
            return Err(Error::Unsupported { proxy, resource });
        };
        if urls_only {
            println!("{url}");
        } else {
            let size = humansize::format_size(asset.size, humansize::BINARY);
            println!("{size:>width$}  {}", asset.name);
        }
    }
    Ok(())
}

/// Parse `owner/repo` or `owner/repo@tag`.
fn parse_repo_spec(spec: &str) -> Result<(String, String, Option<String>), Error> {
    let (repo, tag) = match spec.split_once('@') {
        Some((repo, tag)) if !tag.is_empty() => (repo, Some(tag.to_string())),
        Some(_) => return Err(Error::InvalidSpec(spec.to_string())),
        None => (spec, None),
    };
    let (owner, repo) =
        split_owner_repo(repo).ok_or_else(|| Error::InvalidSpec(spec.to_string()))?;
    Ok((owner.to_string(), repo.to_string(), tag))
}

/// What `fetch` resolved its `SPEC` to.
#[derive(Debug, PartialEq, Eq)]
enum FetchSpec {
    /// A fully determined resource (from a GitHub URL).
    Resource(Resource),
    /// `owner/repo` with an optional tag and a required asset name.
    Release {
        owner: String,
        repo: String,
        tag: Option<String>,
        name: String,
    },
}

/// Parse a `fetch` spec.
///
/// A GitHub URL is decomposed so `--proxy` still applies. Otherwise the spec is
/// `owner/repo[@tag]` with the asset name taken from `name` or from the trailing
/// `/asset`. Omitting the tag uses the latest release.
///
/// When `name` is given, everything after `@` is the tag — no splitting — so
/// tags containing `/` (`release/1.0`) work exactly as written. Without it, the
/// last `/`-segment is taken as the asset name.
fn parse_spec(spec: &str, name: Option<&str>) -> Result<FetchSpec, Error> {
    if spec.starts_with("http://") || spec.starts_with("https://") {
        return spec
            .parse::<Resource>()
            .map(FetchSpec::Resource)
            .map_err(|_| Error::InvalidSpec(spec.to_string()));
    }

    let (repo, tag) = match spec.split_once('@') {
        Some((repo, tag)) if !tag.is_empty() => (repo, Some(tag)),
        Some(_) => return Err(Error::InvalidSpec(spec.to_string())),
        None => (spec, None),
    };

    // With an explicit name, everything after `@` is the tag — a tag may
    // legitimately contain `/`, so splitting it would corrupt it.
    let (tag, inline_name) = match (tag, name.is_some()) {
        (Some(tag), true) => (Some(tag.to_string()), None),
        (Some(tag), false) => match tag.rsplit_once('/') {
            Some((tag, asset)) if !tag.is_empty() && !asset.is_empty() => {
                (Some(tag.to_string()), Some(asset))
            }
            _ => return Err(Error::InvalidSpec(spec.to_string())),
        },
        (None, _) => (None, None),
    };
    let name = name
        .or(inline_name)
        .ok_or_else(|| Error::InvalidSpec(spec.to_string()))?;

    let (owner, repo) =
        split_owner_repo(repo).ok_or_else(|| Error::InvalidSpec(spec.to_string()))?;
    Ok(FetchSpec::Release {
        owner: owner.to_string(),
        repo: repo.to_string(),
        tag,
        name: name.to_string(),
    })
}

/// Split `owner/repo`, rejecting any extra path segments.
fn split_owner_repo(input: &str) -> Option<(&str, &str)> {
    let (owner, repo) = input.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/')).then_some((owner, repo))
}

/// Subset of the `releases/latest` response we need.
#[derive(serde::Deserialize)]
struct LatestRelease {
    tag_name: String,
}

/// Tag of the repository's latest published release.
fn latest_tag(owner: &str, repo: &str) -> Result<String, Error> {
    let api = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let release: LatestRelease = block_on(async {
        let token = crate::token::detect().await;
        fetch::json(
            &api,
            Options {
                token: token.as_deref(),
                timeout: None,
            },
        )
        .await
    })?;

    if release.tag_name.is_empty() {
        return Err(Error::NoRelease(format!("{owner}/{repo}")));
    }
    Ok(release.tag_name)
}

/// Download `spec` to `output`, or to its file name in the current directory.
fn fetch_resource(
    spec: &str,
    name: Option<&str>,
    proxy: Proxy,
    output: Option<PathBuf>,
) -> Result<(), Error> {
    let resource = match parse_spec(spec, name)? {
        FetchSpec::Resource(resource) => resource,
        FetchSpec::Release {
            owner,
            repo,
            tag,
            name,
        } => {
            let tag = match tag {
                Some(tag) => tag,
                None => latest_tag(&owner, &repo)?,
            };
            Resource::release(owner, repo, tag, name)
        }
    };
    let url = resource.url(&proxy).ok_or_else(|| Error::Unsupported {
        proxy,
        resource: resource.clone(),
    })?;

    let destination = match output {
        // A directory (existing, or ending in a separator) receives the name.
        Some(path) if path.is_dir() => path.join(resource.file_name()),
        Some(path) => path,
        None => PathBuf::from(resource.file_name()),
    };

    let bytes = block_on(async {
        let token = crate::token::detect().await;
        fetch::bytes(
            &url,
            Options {
                token: token.as_deref(),
                timeout: None,
            },
        )
        .await
    })?;

    write(&destination, &bytes)?;
    eprintln!("{} -> {}", url, destination.display());
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    std::fs::write(path, bytes).map_err(|source| Error::Write {
        path: path.display().to_string(),
        source,
    })
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to start the runtime")
        .block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_ls_and_fetch() {
        let cli = Cli::try_parse_from(["github-proxy", "ls"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Ls {
                repo: None,
                name: None,
                url: false
            }
        ));

        let cli = Cli::try_parse_from(["github-proxy", "ls", "o/r@v1"]).unwrap();
        assert!(matches!(cli.command, Command::Ls { repo: Some(_), .. }));

        let cli = Cli::try_parse_from(["github-proxy", "fetch", "o/r@v1/a.zip"]).unwrap();
        match cli.command {
            Command::Fetch { spec, output, .. } => {
                assert_eq!(spec, "o/r@v1/a.zip");
                assert!(output.is_none());
            }
            _ => panic!("expected fetch"),
        }
    }

    #[test]
    fn parses_output_flag() {
        let cli = Cli::try_parse_from(["github-proxy", "fetch", "o/r@v1/a.zip", "-o", "out/a.zip"])
            .unwrap();
        match cli.command {
            Command::Fetch { output, .. } => assert_eq!(output, Some(PathBuf::from("out/a.zip"))),
            _ => panic!("expected fetch"),
        }
    }

    #[test]
    fn parses_positional_asset_name() {
        let cli =
            Cli::try_parse_from(["github-proxy", "fetch", "o/r@release/1.0", "a.zip"]).unwrap();
        match cli.command {
            Command::Fetch { spec, name, output } => {
                assert_eq!(spec, "o/r@release/1.0");
                assert_eq!(name.as_deref(), Some("a.zip"));
                assert!(output.is_none());
            }
            _ => panic!("expected fetch"),
        }
    }

    #[test]
    fn parses_repo_specs() {
        assert_eq!(
            parse_repo_spec("o/r").unwrap(),
            ("o".into(), "r".into(), None)
        );
        assert_eq!(
            parse_repo_spec("o/r@v1").unwrap(),
            ("o".into(), "r".into(), Some("v1".into()))
        );
        assert!(parse_repo_spec("o").is_err());
        assert!(parse_repo_spec("o/r@").is_err());
        assert!(parse_repo_spec("o/r/s@v1").is_err());
    }

    #[test]
    fn parses_fetch_specs() {
        fn release(spec: &str, name: Option<&str>) -> (String, String, Option<String>, String) {
            match parse_spec(spec, name).unwrap() {
                FetchSpec::Release {
                    owner,
                    repo,
                    tag,
                    name,
                } => (owner, repo, tag, name),
                FetchSpec::Resource(_) => panic!("expected a release spec"),
            }
        }

        assert_eq!(
            release("o/r@v1/a.zip", None),
            ("o".into(), "r".into(), Some("v1".into()), "a.zip".into())
        );
        // Tags containing `/` stay intact: only the last segment is the name.
        assert_eq!(
            release("o/r@release/1.0/a.zip", None),
            (
                "o".into(),
                "r".into(),
                Some("release/1.0".into()),
                "a.zip".into()
            )
        );
        // A separate name means the tag is everything after `@`, so a tag with
        // `/` is unambiguous.
        assert_eq!(
            release("o/r@release/1.0", Some("a.zip")),
            (
                "o".into(),
                "r".into(),
                Some("release/1.0".into()),
                "a.zip".into()
            )
        );
        // The name argument is a selector, not a suffix to strip.
        assert_eq!(
            release("o/r@v1/ignored.zip", Some("real.zip")),
            (
                "o".into(),
                "r".into(),
                Some("v1/ignored.zip".into()),
                "real.zip".into()
            )
        );
        // No tag: the latest release is used.
        assert_eq!(
            release("o/r", Some("a.zip")),
            ("o".into(), "r".into(), None, "a.zip".into())
        );

        assert_eq!(
            parse_spec("https://github.com/o/r/raw/main/x.sh", None).unwrap(),
            FetchSpec::Resource(Resource::file("o", "r", "main", "x.sh"))
        );
        assert_eq!(
            parse_spec("https://github.com/o/r/releases/download/v1/a.zip", None).unwrap(),
            FetchSpec::Resource(Resource::release("o", "r", "v1", "a.zip"))
        );
        // The asset name is required once a tag is omitted.
        assert!(parse_spec("o/r", None).is_err());
        assert!(parse_spec("o/r@v1", None).is_err());
        assert!(parse_spec("https://example.com/x", None).is_err());
    }

    #[test]
    fn output_defaults_to_asset_name() {
        // Only the naming rule is checked; no download happens.
        assert_eq!(
            Resource::release("o", "r", "v1", "a.zip").file_name(),
            "a.zip"
        );
        assert_eq!(
            Resource::file("o", "r", "main", "bin/tool").file_name(),
            "tool"
        );
    }
}
