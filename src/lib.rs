pub mod cli;
mod error;
mod proxy;
mod resource;
pub use error::ConversionError;
pub use proxy::Proxy;
pub use resource::Resource;

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn test_file_resource_xget() {
        let resource = Resource::file(
            "easy-install".to_string(),
            "easy-install".to_string(),
            "main".to_string(),
            "install.sh".to_string(),
        );
        let url = resource.url(&Proxy::Xget).unwrap();
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/easy-install/easy-install/raw/main/install.sh"
        );
    }

    #[test]
    fn test_file_resource_gh_proxy() {
        let resource = Resource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.url(&Proxy::GhProxy).unwrap();
        assert_eq!(
            url,
            "https://gh-proxy.com/https://github.com/owner/repo/raw/main/file.sh"
        );
    }

    #[test]
    fn test_file_resource_jsdelivr() {
        let resource = Resource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.js".to_string(),
        );
        let url = resource.url(&Proxy::Jsdelivr).unwrap();
        assert_eq!(url, "https://cdn.jsdelivr.net/gh/owner/repo@main/file.js");
    }

    #[test]
    fn test_file_resource_github() {
        let resource = Resource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.url(&Proxy::Github).unwrap();
        assert_eq!(url, "https://github.com/owner/repo/raw/main/file.sh");
    }

    #[test]
    fn test_release_resource_xget() {
        let resource = Resource::release(
            "easy-install".to_string(),
            "easy-install".to_string(),
            "nightly".to_string(),
            "ei-aarch64-apple-darwin.tar.gz".to_string(),
        );
        let url = resource.url(&Proxy::Xget).unwrap();
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/easy-install/easy-install/releases/download/nightly/ei-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn test_release_resource_gh_proxy() {
        let resource = Resource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.url(&Proxy::GhProxy).unwrap();
        assert_eq!(
            url,
            "https://gh-proxy.com/https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz"
        );
    }

    #[test]
    fn test_release_resource_jsdelivr_not_supported() {
        let resource = Resource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.url(&Proxy::Jsdelivr);
        assert!(url.is_none(), "jsdelivr should not support release assets");
    }

    #[test]
    fn test_release_resource_github() {
        let resource = Resource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.url(&Proxy::Github).unwrap();
        assert_eq!(
            url,
            "https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz"
        );
    }

    #[test]
    fn test_proxy_type_from_str() {
        assert_eq!(Proxy::from_str("github").unwrap(), Proxy::Github);
        assert_eq!(Proxy::from_str("gh-proxy").unwrap(), Proxy::GhProxy);
        assert_eq!(Proxy::from_str("xget").unwrap(), Proxy::Xget);
        assert_eq!(Proxy::from_str("jsdelivr").unwrap(), Proxy::Jsdelivr);
        assert_eq!(Proxy::from_str("statically").unwrap(), Proxy::Statically);
        assert_eq!(Proxy::from_str("XGET").unwrap(), Proxy::Xget);
        assert!(Proxy::from_str("invalid").is_err());
    }

    #[test]
    fn test_file_resource_statically() {
        let resource = Resource::file(
            "easy-install".to_string(),
            "easy-install".to_string(),
            "main".to_string(),
            "install.sh".to_string(),
        );
        let url = resource.url(&Proxy::Statically).unwrap();
        assert_eq!(
            url,
            "https://cdn.statically.io/gh/easy-install/easy-install/main/install.sh"
        );
    }

    #[test]
    fn test_release_resource_statically_not_supported() {
        let resource = Resource::release(
            "owner".to_string(),
            "repo".to_string(),
            "v1.0.0".to_string(),
            "app.tar.gz".to_string(),
        );
        let url = resource.url(&Proxy::Statically);
        assert!(
            url.is_none(),
            "statically should not support release assets"
        );
    }

    #[test]
    fn test_nested_file_path() {
        let resource = Resource::file(
            "owner".to_string(),
            "repo".to_string(),
            "main".to_string(),
            "src/lib/file.rs".to_string(),
        );
        let url = resource.url(&Proxy::Xget).unwrap();
        assert_eq!(
            url,
            "https://xget.xi-xu.me/gh/owner/repo/raw/main/src/lib/file.rs"
        );
    }

    #[test]
    fn test_branch_with_refs() {
        let resource = Resource::file(
            "owner".to_string(),
            "repo".to_string(),
            "refs/heads/main".to_string(),
            "file.sh".to_string(),
        );
        let url = resource.url(&Proxy::Github).unwrap();
        assert_eq!(
            url,
            "https://github.com/owner/repo/raw/refs/heads/main/file.sh"
        );
    }

    #[test]
    fn test_parse_raw_file_url() {
        let url = "https://github.com/easy-install/easy-install/raw/main/install.sh";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::file(
                "easy-install".to_string(),
                "easy-install".to_string(),
                "main".to_string(),
                "install.sh".to_string()
            )
        );
    }

    #[test]
    fn test_parse_blob_file_url() {
        let url = "https://github.com/owner/repo/blob/main/src/lib.rs";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::file(
                "owner".to_string(),
                "repo".to_string(),
                "main".to_string(),
                "src/lib.rs".to_string()
            )
        );
    }

    #[test]
    fn test_parse_release_download_url() {
        let url = "https://github.com/easy-install/easy-install/releases/download/nightly/ei-aarch64-apple-darwin.tar.gz";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::release(
                "easy-install".to_string(),
                "easy-install".to_string(),
                "nightly".to_string(),
                "ei-aarch64-apple-darwin.tar.gz".to_string()
            )
        );
    }

    #[test]
    fn test_parse_raw_file_with_refs() {
        let url = "https://github.com/owner/repo/raw/refs/heads/main/file.sh";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::file(
                "owner".to_string(),
                "repo".to_string(),
                "refs/heads/main".to_string(),
                "file.sh".to_string()
            )
        );
    }

    #[test]
    fn test_parse_nested_path() {
        let url = "https://github.com/owner/repo/raw/main/src/lib/file.rs";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::file(
                "owner".to_string(),
                "repo".to_string(),
                "main".to_string(),
                "src/lib/file.rs".to_string()
            )
        );
    }

    #[test]
    fn test_parse_invalid_url() {
        let url = "https://example.com/file.sh";
        let result = Resource::try_from(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_http_url() {
        let url = "http://github.com/owner/repo/raw/main/file.sh";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::file(
                "owner".to_string(),
                "repo".to_string(),
                "main".to_string(),
                "file.sh".to_string()
            )
        );
    }
    #[test]
    fn test_parse_fish() {
        let url = "https://github.com/fish-shell/fish-shell/releases/download/4.1.2/fish-4.1.2-linux-aarch64.tar.xz";
        let resource = Resource::try_from(url).unwrap();
        assert_eq!(
            resource,
            Resource::release(
                "fish-shell".to_string(),
                "fish-shell".to_string(),
                "4.1.2".to_string(),
                "fish-4.1.2-linux-aarch64.tar.xz".to_string()
            )
        );
    }
}
