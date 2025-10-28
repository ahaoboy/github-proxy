use strum_macros::EnumIter;

use crate::proxy::Proxy;

/// GitHub resource types
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(EnumIter, Debug, PartialEq, Hash, Eq, Clone)]
pub enum GitHubResource {
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

impl GitHubResource {
    /// Create a new file resource
    ///
    /// # Arguments
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    /// * `reference` - Git reference (branch, tag, commit hash, or refs/heads/branch)
    /// * `path` - File path in the repository
    pub fn file(owner: String, repo: String, reference: String, path: String) -> Self {
        GitHubResource::File {
            owner,
            repo,
            reference,
            path,
        }
    }

    /// Create a new release resource
    pub fn release(owner: String, repo: String, tag: String, name: String) -> Self {
        GitHubResource::Release {
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
            GitHubResource::File {
                owner,
                repo,
                reference,
                path,
            } => Some(match proxy_type {
                Proxy::GitHub => {
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
            }),
            GitHubResource::Release {
                owner,
                repo,
                tag,
                name,
            } => match proxy_type {
                Proxy::GitHub => Some(format!(
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
            },
        }
    }
}
