use crate::proxy::ProxyType;

/// GitHub resource types
#[derive(Debug, Clone, PartialEq)]
pub enum GitHubResource {
    /// Raw file in a repository
    /// Format: owner/repo/branch/path
    File {
        owner: String,
        repo: String,
        branch: String,
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
    pub fn file(owner: String, repo: String, branch: String, path: String) -> Self {
        GitHubResource::File {
            owner,
            repo,
            branch,
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
    pub fn to_url(&self, proxy_type: &ProxyType) -> String {
        match self {
            GitHubResource::File {
                owner,
                repo,
                branch,
                path,
            } => match proxy_type {
                ProxyType::GitHub => {
                    format!(
                        "https://github.com/{}/{}/raw/{}/{}",
                        owner, repo, branch, path
                    )
                }
                ProxyType::Xget => {
                    format!(
                        "https://xget.xi-xu.me/gh/{}/{}/raw/{}/{}",
                        owner, repo, branch, path
                    )
                }
                ProxyType::GhProxy => {
                    format!(
                        "https://gh-proxy.com/https://github.com/{}/{}/raw/{}/{}",
                        owner, repo, branch, path
                    )
                }
                ProxyType::Jsdelivr => {
                    format!(
                        "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
                        owner, repo, branch, path
                    )
                }
            },
            GitHubResource::Release {
                owner,
                repo,
                tag,
                name,
            } => match proxy_type {
                ProxyType::GitHub => {
                    format!(
                        "https://github.com/{}/{}/releases/download/{}/{}",
                        owner, repo, tag, name
                    )
                }
                ProxyType::Xget => {
                    format!(
                        "https://xget.xi-xu.me/gh/{}/{}/releases/download/{}/{}",
                        owner, repo, tag, name
                    )
                }
                ProxyType::GhProxy => {
                    format!(
                        "https://gh-proxy.com/https://github.com/{}/{}/releases/download/{}/{}",
                        owner, repo, tag, name
                    )
                }
                ProxyType::Jsdelivr => {
                    format!(
                        "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
                        owner, repo, tag, name
                    )
                }
            },
        }
    }
}
