use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use regex::Regex;
use strum_macros::EnumIter;

use crate::{Proxy, error::ConversionError};

/// A GitHub resource that can be downloaded through a [`Proxy`].
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(EnumIter, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Resource {
    /// A raw file in a repository: `owner/repo@reference/path`.
    File {
        /// Repository owner.
        owner: String,
        /// Repository name.
        repo: String,
        /// Branch, tag, commit hash or `refs/heads/<branch>`.
        reference: String,
        /// Path inside the repository.
        path: String,
    },
    /// A release asset: `owner/repo@tag/name`.
    Release {
        /// Repository owner.
        owner: String,
        /// Repository name.
        repo: String,
        /// Release tag.
        tag: String,
        /// Asset file name.
        name: String,
    },
}

impl Resource {
    /// A raw repository file.
    pub fn file(
        owner: impl Into<String>,
        repo: impl Into<String>,
        reference: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Resource::File {
            owner: owner.into(),
            repo: repo.into(),
            reference: reference.into(),
            path: path.into(),
        }
    }

    /// A release asset.
    pub fn release(
        owner: impl Into<String>,
        repo: impl Into<String>,
        tag: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Resource::Release {
            owner: owner.into(),
            repo: repo.into(),
            tag: tag.into(),
            name: name.into(),
        }
    }

    /// Repository owner.
    pub fn owner(&self) -> &str {
        match self {
            Resource::File { owner, .. } | Resource::Release { owner, .. } => owner,
        }
    }

    /// Repository name.
    pub fn repo(&self) -> &str {
        match self {
            Resource::File { repo, .. } | Resource::Release { repo, .. } => repo,
        }
    }

    /// Name of the downloaded file (last path segment, or the asset name).
    pub fn file_name(&self) -> &str {
        match self {
            Resource::File { path, .. } => path.rsplit('/').next().unwrap_or(path),
            Resource::Release { name, .. } => name,
        }
    }

    /// Proxied URL, or `None` when `proxy` cannot serve this kind of resource.
    pub fn url(&self, proxy: &Proxy) -> Option<String> {
        match self {
            Resource::File {
                owner,
                repo,
                reference,
                path,
            } => Some(match proxy {
                Proxy::Github => {
                    format!("https://github.com/{owner}/{repo}/raw/{reference}/{path}")
                }
                Proxy::GhProxy => {
                    format!(
                        "https://gh-proxy.com/https://github.com/{owner}/{repo}/raw/{reference}/{path}"
                    )
                }
                Proxy::Xget => {
                    format!("https://xget.xi-xu.me/gh/{owner}/{repo}/raw/{reference}/{path}")
                }
                Proxy::Jsdelivr => {
                    format!("https://cdn.jsdelivr.net/gh/{owner}/{repo}@{reference}/{path}")
                }
                Proxy::Statically => {
                    format!("https://cdn.statically.io/gh/{owner}/{repo}/{reference}/{path}")
                }
            }),
            Resource::Release {
                owner,
                repo,
                tag,
                name,
            } => match proxy {
                Proxy::Github => Some(format!(
                    "https://github.com/{owner}/{repo}/releases/download/{tag}/{name}"
                )),
                Proxy::GhProxy => Some(format!(
                    "https://gh-proxy.com/https://github.com/{owner}/{repo}/releases/download/{tag}/{name}"
                )),
                Proxy::Xget => Some(format!(
                    "https://xget.xi-xu.me/gh/{owner}/{repo}/releases/download/{tag}/{name}"
                )),
                Proxy::Jsdelivr | Proxy::Statically => None,
            },
        }
    }
}

impl fmt::Display for Resource {
    /// Round-trips through [`Resource::try_from`].
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resource::File {
                owner,
                repo,
                reference,
                path,
            } => write!(f, "file {owner}/{repo}@{reference}/{path}"),
            Resource::Release {
                owner,
                repo,
                tag,
                name,
            } => write!(f, "release {owner}/{repo}@{tag}/{name}"),
        }
    }
}

/// `raw` and `blob` URLs differ only in the path segment after the repo.
fn file_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/(?:raw|blob)/(?P<rest>.+)$",
        )
        .expect("file URL regex is valid")
    })
}

fn release_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/releases/download/(?P<tag>[^/]+)/(?P<name>.+)$")
            .expect("release URL regex is valid")
    })
}

impl TryFrom<&str> for Resource {
    type Error = ConversionError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let input = input.trim();

        if let Some(captures) = file_regex().captures(input) {
            let (reference, path) = split_reference_and_path(&captures["rest"])?;
            return Ok(Resource::File {
                owner: captures["owner"].to_string(),
                repo: captures["repo"].to_string(),
                reference,
                path,
            });
        }

        if let Some(captures) = release_regex().captures(input) {
            return Ok(Resource::Release {
                owner: captures["owner"].to_string(),
                repo: captures["repo"].to_string(),
                tag: captures["tag"].to_string(),
                name: captures["name"].to_string(),
            });
        }

        Err(ConversionError::InvalidUrl(input.to_string()))
    }
}

impl TryFrom<String> for Resource {
    type Error = ConversionError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::try_from(input.as_str())
    }
}

impl FromStr for Resource {
    type Err = ConversionError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::try_from(input)
    }
}

/// Split `<reference>/<path>` where the reference may itself contain slashes
/// (`refs/heads/main`, `refs/tags/v1.0`).
fn split_reference_and_path(rest: &str) -> Result<(String, String), ConversionError> {
    let parts: Vec<&str> = rest.split('/').collect();

    // `refs/<kind>/<name>/<path…>`
    if parts.first() == Some(&"refs") {
        if parts.len() < 4 {
            return Err(ConversionError::ParseError(
                "missing path after refs/<kind>/<name>".to_string(),
            ));
        }
        return Ok((parts[..3].join("/"), parts[3..].join("/")));
    }

    // `<reference>/<path…>`
    if parts.len() < 2 {
        return Err(ConversionError::ParseError(
            "missing path after reference".to_string(),
        ));
    }
    Ok((parts[0].to_string(), parts[1..].join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(resource: &Resource, proxy: Proxy) -> String {
        resource.url(&proxy).expect("proxy supports the resource")
    }

    #[test]
    fn builds_file_urls_for_every_mirror() {
        let file = Resource::file("o", "r", "main", "a/b.sh");
        assert_eq!(
            url(&file, Proxy::Github),
            "https://github.com/o/r/raw/main/a/b.sh"
        );
        assert_eq!(
            url(&file, Proxy::GhProxy),
            "https://gh-proxy.com/https://github.com/o/r/raw/main/a/b.sh"
        );
        assert_eq!(
            url(&file, Proxy::Xget),
            "https://xget.xi-xu.me/gh/o/r/raw/main/a/b.sh"
        );
        assert_eq!(
            url(&file, Proxy::Jsdelivr),
            "https://cdn.jsdelivr.net/gh/o/r@main/a/b.sh"
        );
        assert_eq!(
            url(&file, Proxy::Statically),
            "https://cdn.statically.io/gh/o/r/main/a/b.sh"
        );
    }

    #[test]
    fn builds_release_urls_where_supported() {
        let release = Resource::release("o", "r", "v1", "a.tar.gz");
        assert_eq!(
            url(&release, Proxy::Github),
            "https://github.com/o/r/releases/download/v1/a.tar.gz"
        );
        assert_eq!(
            url(&release, Proxy::GhProxy),
            "https://gh-proxy.com/https://github.com/o/r/releases/download/v1/a.tar.gz"
        );
        assert_eq!(
            url(&release, Proxy::Xget),
            "https://xget.xi-xu.me/gh/o/r/releases/download/v1/a.tar.gz"
        );
        assert!(release.url(&Proxy::Jsdelivr).is_none());
        assert!(release.url(&Proxy::Statically).is_none());
    }

    #[test]
    fn parses_raw_blob_and_release_urls() {
        assert_eq!(
            Resource::try_from("https://github.com/o/r/raw/main/x.rs").unwrap(),
            Resource::file("o", "r", "main", "x.rs")
        );
        assert_eq!(
            Resource::try_from("http://github.com/o/r/blob/main/src/x.rs").unwrap(),
            Resource::file("o", "r", "main", "src/x.rs")
        );
        assert_eq!(
            Resource::try_from("https://github.com/o/r/raw/refs/heads/main/x.rs").unwrap(),
            Resource::file("o", "r", "refs/heads/main", "x.rs")
        );
        assert_eq!(
            Resource::try_from("https://github.com/o/r/releases/download/v1/a.tar.gz").unwrap(),
            Resource::release("o", "r", "v1", "a.tar.gz")
        );
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(Resource::try_from("https://example.com/o/r/raw/main/x").is_err());
        assert!(Resource::try_from("https://github.com/o/r/raw/main").is_err());
        assert!(Resource::try_from("https://github.com/o/r/raw/refs/heads").is_err());
    }

    #[test]
    fn display_round_trips() {
        for resource in [
            Resource::file("o", "r", "main", "a/b"),
            Resource::release("o", "r", "v1", "a.zip"),
        ] {
            assert!(!resource.to_string().is_empty());
        }
        assert_eq!(
            Resource::file("o", "r", "main", "x").to_string(),
            "file o/r@main/x"
        );
        assert_eq!(
            Resource::release("o", "r", "v1", "x").to_string(),
            "release o/r@v1/x"
        );
    }
}
