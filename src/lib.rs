//! GitHub Proxy URL Generator
//!
//! This library provides functionality to generate proxied URLs for GitHub resources
//! (raw files and release assets) using various proxy services.
//!
//! # Examples
//!
//! ```
//! use github_proxy::{GitHubResource, ProxyType};
//!
//! // Create a file resource
//! let resource = GitHubResource::file(
//!     "owner".to_string(),
//!     "repo".to_string(),
//!     "main".to_string(),
//!     "file.sh".to_string()
//! );
//!
//! // Generate URL with xget proxy
//! let proxy = ProxyType::Xget;
//! let url = resource.to_url(&proxy);
//! println!("{}", url);
//! ```

pub mod cli;
mod error;
mod proxy;
mod resource;

pub use error::ConversionError;
pub use proxy::ProxyType;
pub use resource::GitHubResource;

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn test_file_resource_xget() {
        let resource = GitHubResource::file(
            "easy-install".to_string(),
            "easy-install".to_string(),
            "main".to_string(),
            "install.sh".to_string(),
        );
        let url = resource.to_url(&ProxyType::Xget);
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/easy-install/easy-install/raw/main/install.sh"
        );
    }

    #[test]
    fn test_file_resource_gh_proxy() {
        let resource = GitHubResource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.to_url(&ProxyType::GhProxy);
        assert_eq!(
            url,
            "https://gh-proxy.com/https://github.com/owner/repo/raw/main/file.sh"
        );
    }

    #[test]
    fn test_file_resource_jsdelivr() {
        let resource = GitHubResource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.js".to_string(),
        );
        let url = resource.to_url(&ProxyType::Jsdelivr);
        assert_eq!(url, "https://cdn.jsdelivr.net/gh/owner/repo@main/file.js");
    }

    #[test]
    fn test_file_resource_github() {
        let resource = GitHubResource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.to_url(&ProxyType::GitHub);
        assert_eq!(url, "https://github.com/owner/repo/raw/main/file.sh");
    }

    #[test]
    fn test_release_resource_xget() {
        let resource = GitHubResource::release(
            "easy-install".to_string(),
            "easy-install".to_string(),
            "nightly".to_string(),
            "ei-aarch64-apple-darwin.tar.gz".to_string(),
        );
        let url = resource.to_url(&ProxyType::Xget);
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/easy-install/easy-install/releases/download/nightly/ei-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn test_release_resource_gh_proxy() {
        let resource = GitHubResource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.to_url(&ProxyType::GhProxy);
        assert_eq!(
            url,
            "https://gh-proxy.com/https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz"
        );
    }

    #[test]
    fn test_release_resource_jsdelivr() {
        let resource = GitHubResource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.to_url(&ProxyType::Jsdelivr);
        assert_eq!(
            url,
            "https://cdn.jsdelivr.net/gh/owner/repo@v1.0.0/app.tar.gz"
        );
    }

    #[test]
    fn test_release_resource_github() {
        let resource = GitHubResource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.to_url(&ProxyType::GitHub);
        assert_eq!(
            url,
            "https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz"
        );
    }

    #[test]
    fn test_proxy_type_from_str() {
        assert_eq!(ProxyType::from_str("github").unwrap(), ProxyType::GitHub);
        assert_eq!(ProxyType::from_str("gh-proxy").unwrap(), ProxyType::GhProxy);
        assert_eq!(ProxyType::from_str("xget").unwrap(), ProxyType::Xget);
        assert_eq!(
            ProxyType::from_str("jsdelivr").unwrap(),
            ProxyType::Jsdelivr
        );
        assert_eq!(ProxyType::from_str("XGET").unwrap(), ProxyType::Xget);
        assert!(ProxyType::from_str("invalid").is_err());
    }

    #[test]
    fn test_nested_file_path() {
        let resource = GitHubResource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "src/lib/file.rs".to_string(),
        );
        let url = resource.to_url(&ProxyType::Xget);
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/owner/repo/raw/main/src/lib/file.rs"
        );
    }

    #[test]
    fn test_branch_with_refs() {
        let resource = GitHubResource::file(
            "owner".to_string(),
            "repo".to_string(),
            "refs/heads/main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.to_url(&ProxyType::GitHub);
        assert_eq!(
            url,
            "https://github.com/owner/repo/raw/refs/heads/main/file.sh"
        );
    }
}
