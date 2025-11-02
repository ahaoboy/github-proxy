use crate::proxy::Proxy;
use strum_macros::EnumIter;

/// Github resource types
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(EnumIter, Debug, PartialEq, Hash, Eq, Clone)]
pub enum Resource {
    /// Raw file in a repository
    /// Format: owner/repo/reference/path
    /// reference can be: branch name, tag, commit hash, or refs/heads/branch
    File {
        owner: String,
        repo: String,
        reference: String,
        path: String,
    },
    /// Release asset
    /// Format: owner/repo/tag/filename
    Release {
        owner: String,
        repo: String,
        tag: String,
        name: String,
    },
}

impl Resource {
    /// Create a new file resource
    ///
    /// # Arguments
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `reference` - Git reference (branch, tag, commit hash, or refs/heads/branch)
    /// * `path` - File path in the repository
    pub fn file(owner: String, repo: String, reference: String, path: String) -> Self {
        Resource::File {
            owner,
            repo,
            reference,
            path,
        }
    }

    /// Create a new release resource
    pub fn release(owner: String, repo: String, tag: String, name: String) -> Self {
        Resource::Release {
            owner,
            repo,
            tag,
            name,
        }
    }

    /// Convert the resource to a proxied URL
    ///
    /// Returns None if the proxy type doesn't support the resource type
    /// (e.g., jsdelivr doesn't support release assets from /releases/download/)
    pub fn url(&self, proxy_type: &Proxy) -> Option<String> {
        match self {
            Resource::File {
                owner,
                repo,
                reference,
                path,
            } => Some(match proxy_type {
                Proxy::Github => {
                    format!(
                        "https://github.com/{}/{}/raw/{}/{}",
                        owner, repo, reference, path
                    )
                }
                Proxy::Xget => {
                    format!(
                        "https://xget.xi-xu.me/gh/{}/{}/raw/{}/{}",
                        owner, repo, reference, path
                    )
                }
                Proxy::GhProxy => {
                    format!(
                        "https://gh-proxy.com/https://github.com/{}/{}/raw/{}/{}",
                        owner, repo, reference, path
                    )
                }
                Proxy::Jsdelivr => {
                    format!(
                        "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
                        owner, repo, reference, path
                    )
                }
                Proxy::Statically => {
                    format!(
                        "https://cdn.statically.io/gh/{}/{}/{}/{}",
                        owner, repo, reference, path
                    )
                }
            }),
            Resource::Release {
                owner,
                repo,
                tag,
                name,
            } => match proxy_type {
                Proxy::Github => Some(format!(
                    "https://github.com/{}/{}/releases/download/{}/{}",
                    owner, repo, tag, name
                )),
                Proxy::Xget => Some(format!(
                    "https://xget.xi-xu.me/gh/{}/{}/releases/download/{}/{}",
                    owner, repo, tag, name
                )),
                Proxy::GhProxy => Some(format!(
                    "https://gh-proxy.com/https://github.com/{}/{}/releases/download/{}/{}",
                    owner, repo, tag, name
                )),
                // jsdelivr doesn't support release assets from /releases/download/
                Proxy::Jsdelivr => None,
                // statically doesn't support release assets from /releases/download/
                Proxy::Statically => None,
            },
        }
    }
}

use crate::error::ConversionError;
use regex::Regex;
use std::sync::OnceLock;

// Lazy static regex patterns
fn raw_file_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Match everything after /raw/ and then split to find the path
        Regex::new(r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/raw/(?P<rest>.+)$")
            .unwrap()
    })
}

fn blob_file_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Match everything after /blob/ and then split to find the path
        Regex::new(r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/blob/(?P<rest>.+)$")
            .unwrap()
    })
}

fn release_download_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^https?://github\.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/releases/download/(?P<tag>[^/]+)/(?P<filename>.+)$")
            .unwrap()
    })
}

impl TryFrom<&str> for Resource {
    type Error = ConversionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = value.trim();

        // Try to match raw file URL: https://github.com/owner/repo/raw/ref/path
        if let Some(captures) = raw_file_regex().captures(value) {
            let owner = captures["owner"].to_string();
            let repo = captures["repo"].to_string();
            let rest = &captures["rest"];

            // Split the rest to separate reference and path
            // We need to handle cases like:
            // - "main/file.sh" -> reference: "main", path: "file.sh"
            // - "refs/heads/main/file.sh" -> reference: "refs/heads/main", path: "file.sh"
            let (reference, path) = split_reference_and_path(rest)?;

            return Ok(Resource::File {
                owner,
                repo,
                reference,
                path,
            });
        }

        // Try to match blob file URL: https://github.com/owner/repo/blob/ref/path
        if let Some(captures) = blob_file_regex().captures(value) {
            let owner = captures["owner"].to_string();
            let repo = captures["repo"].to_string();
            let rest = &captures["rest"];

            let (reference, path) = split_reference_and_path(rest)?;

            return Ok(Resource::File {
                owner,
                repo,
                reference,
                path,
            });
        }

        // Try to match release download URL: https://github.com/owner/repo/releases/download/tag/filename
        if let Some(captures) = release_download_regex().captures(value) {
            return Ok(Resource::Release {
                owner: captures["owner"].to_string(),
                repo: captures["repo"].to_string(),
                tag: captures["tag"].to_string(),
                name: captures["filename"].to_string(),
            });
        }

        Err(ConversionError::InvalidUrl(value.to_string()))
    }
}

/// Split the rest of the URL into reference and path
/// Handles cases like:
/// - "main/file.sh" -> ("main", "file.sh")
/// - "refs/heads/main/file.sh" -> ("refs/heads/main", "file.sh")
/// - "refs/tags/v1.0/file.sh" -> ("refs/tags/v1.0", "file.sh")
fn split_reference_and_path(rest: &str) -> Result<(String, String), ConversionError> {
    let parts: Vec<&str> = rest.split('/').collect();

    if parts.is_empty() {
        return Err(ConversionError::ParseError(
            "Missing reference and path".to_string(),
        ));
    }

    // Check if it starts with "refs/"
    if parts.len() >= 4 && parts[0] == "refs" {
        // Pattern: refs/heads/main/path or refs/tags/v1.0/path
        let reference = format!("{}/{}/{}", parts[0], parts[1], parts[2]);
        let path = parts[3..].join("/");

        if path.is_empty() {
            return Err(ConversionError::ParseError("Missing file path".to_string()));
        }

        Ok((reference, path))
    } else if parts.len() >= 2 {
        // Pattern: main/path or v1.0/path
        let reference = parts[0].to_string();
        let path = parts[1..].join("/");
        Ok((reference, path))
    } else {
        Err(ConversionError::ParseError(
            "Invalid reference/path format".to_string(),
        ))
    }
}

impl TryFrom<String> for Resource {
    type Error = ConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Resource::try_from(value.as_str())
    }
}
