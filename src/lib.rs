use std::{fmt, str::FromStr};

#[derive(Debug, PartialEq)]
pub enum Proxy {
    GitHub,
    #[allow(clippy::should_implement_trait)]
    GhProxy,
    Xget,
    Jsdelivr,
}

impl FromStr for Proxy {
    type Err = ConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(Proxy::GitHub),
            "gh-proxy" => Ok(Proxy::GhProxy),
            "xget" => Ok(Proxy::Xget),
            "jsdelivr" => Ok(Proxy::Jsdelivr),
            _ => Err(ConversionError::InvalidProxyType(s.to_string())),
        }
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Proxy::GitHub => write!(f, "github"),
            Proxy::GhProxy => write!(f, "gh-proxy"),
            Proxy::Xget => write!(f, "xget"),
            Proxy::Jsdelivr => write!(f, "jsdelivr"),
        }
    }
}

#[derive(Debug)]
pub enum ConversionError {
    InvalidUrl(String),
    UnsupportedUrlType(String),
    InvalidProxyType(String),
    ParseError(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConversionError::InvalidUrl(msg) => {
                write!(
                    f,
                    "Invalid URL: {}\nSupported formats:\n  - https://github.com/{{owner}}/{{repo}}/raw/{{ref}}/{{path}}\n  - https://github.com/{{owner}}/{{repo}}/releases/download/{{tag}}/{{filename}}",
                    msg
                )
            }
            ConversionError::UnsupportedUrlType(msg) => {
                write!(
                    f,
                    "Unsupported URL type: {}\nSupported formats:\n  - https://github.com/{{owner}}/{{repo}}/raw/{{ref}}/{{path}}\n  - https://github.com/{{owner}}/{{repo}}/releases/download/{{tag}}/{{filename}}",
                    msg
                )
            }
            ConversionError::InvalidProxyType(msg) => {
                write!(
                    f,
                    "Invalid proxy type: {}\nSupported types: github, gh-proxy, xget, jsdelivr",
                    msg
                )
            }
            ConversionError::ParseError(msg) => {
                write!(f, "Parse error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConversionError {}

#[derive(Debug, PartialEq)]
pub struct RawFile {
    pub owner: String,
    pub repo: String,
    pub reference: String,
    pub path: String,
}

impl RawFile {
    pub fn parse(url: &str) -> Result<Self, ConversionError> {
        if !url.starts_with("https://github.com/") {
            return Err(ConversionError::InvalidUrl(
                "URL must start with 'https://github.com/'".to_string(),
            ));
        }

        // Pattern: https://github.com/{owner}/{repo}/raw/{ref}/{path}
        let after_github = url.trim_start_matches("https://github.com/");

        if !after_github.contains("/raw/") {
            return Err(ConversionError::ParseError(
                "Raw file URL must contain '/raw/' segment".to_string(),
            ));
        }

        let parts: Vec<&str> = after_github.split("/raw/").collect();
        if parts.len() != 2 {
            return Err(ConversionError::ParseError(
                "Invalid raw file URL format".to_string(),
            ));
        }

        // Parse owner/repo from the first part
        let owner_repo: Vec<&str> = parts[0].split('/').collect();
        if owner_repo.len() != 2 {
            return Err(ConversionError::ParseError(
                "Invalid owner/repo format".to_string(),
            ));
        }

        let owner = owner_repo[0].to_string();
        let repo = owner_repo[1].to_string();

        // Parse reference and path from the second part
        let ref_and_path = parts[1];
        let ref_path_parts: Vec<&str> = ref_and_path.split('/').collect();

        if ref_path_parts.is_empty() {
            return Err(ConversionError::ParseError(
                "Missing reference and path".to_string(),
            ));
        }

        // For URLs like /raw/refs/heads/main/install.sh
        // reference is "refs/heads/main" and path is "install.sh"
        let (reference, path) = if ref_path_parts.len() >= 4 && ref_path_parts[0] == "refs" {
            // Pattern: refs/heads/main/path or refs/tags/v1.0/path
            let reference = format!(
                "{}/{}/{}",
                ref_path_parts[0], ref_path_parts[1], ref_path_parts[2]
            );
            let path = ref_path_parts[3..].join("/");
            (reference, path)
        } else if ref_path_parts.len() >= 2 {
            // Pattern: main/path or v1.0/path
            let reference = ref_path_parts[0].to_string();
            let path = ref_path_parts[1..].join("/");
            (reference, path)
        } else {
            return Err(ConversionError::ParseError(
                "Invalid reference/path format".to_string(),
            ));
        };

        if reference.is_empty() || path.is_empty() {
            return Err(ConversionError::ParseError(
                "Missing reference or path".to_string(),
            ));
        }

        Ok(RawFile {
            owner,
            repo,
            reference,
            path,
        })
    }

    pub fn url(&self, proxy: &Proxy) -> String {
        match proxy {
            Proxy::GitHub => {
                format!(
                    "https://github.com/{}/{}/raw/{}/{}",
                    self.owner, self.repo, self.reference, self.path
                )
            }
            Proxy::Xget => {
                format!(
                    "https://xget.xi-xu.me/gh/{}/{}/raw/{}/{}",
                    self.owner, self.repo, self.reference, self.path
                )
            }
            Proxy::GhProxy => {
                format!(
                    "https://gh-proxy.com/https://github.com/{}/{}/raw/{}/{}",
                    self.owner, self.repo, self.reference, self.path
                )
            }
            Proxy::Jsdelivr => {
                format!(
                    "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
                    self.owner, self.repo, self.reference, self.path
                )
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ReleaseAsset {
    pub owner: String,
    pub repo: String,
    pub tag: String,
    pub filename: String,
}

impl ReleaseAsset {
    pub fn parse(url: &str) -> Result<Self, ConversionError> {
        if !url.starts_with("https://github.com/") {
            return Err(ConversionError::InvalidUrl(
                "URL must start with 'https://github.com/'".to_string(),
            ));
        }

        // Pattern: https://github.com/{owner}/{repo}/releases/download/{tag}/{filename}
        let after_github = url.trim_start_matches("https://github.com/");

        if !after_github.contains("/releases/download/") {
            return Err(ConversionError::ParseError(
                "Release asset URL must contain '/releases/download/' segment".to_string(),
            ));
        }

        let parts: Vec<&str> = after_github.split("/releases/download/").collect();
        if parts.len() != 2 {
            return Err(ConversionError::ParseError(
                "Invalid release asset URL format".to_string(),
            ));
        }

        // Parse owner/repo from the first part
        let owner_repo: Vec<&str> = parts[0].split('/').collect();
        if owner_repo.len() != 2 {
            return Err(ConversionError::ParseError(
                "Invalid owner/repo format".to_string(),
            ));
        }

        let owner = owner_repo[0].to_string();
        let repo = owner_repo[1].to_string();

        // Parse tag and filename from the second part
        let tag_and_filename = parts[1];
        let tag_filename_parts: Vec<&str> = tag_and_filename.split('/').collect();

        if tag_filename_parts.len() != 2 {
            return Err(ConversionError::ParseError(
                "Invalid tag/filename format".to_string(),
            ));
        }

        let tag = tag_filename_parts[0].to_string();
        let filename = tag_filename_parts[1].to_string();

        if tag.is_empty() || filename.is_empty() {
            return Err(ConversionError::ParseError(
                "Missing tag or filename".to_string(),
            ));
        }

        Ok(ReleaseAsset {
            owner,
            repo,
            tag,
            filename,
        })
    }

    pub fn url(&self, proxy: &Proxy) -> String {
        match proxy {
            Proxy::GitHub => {
                format!(
                    "https://github.com/{}/{}/releases/download/{}/{}",
                    self.owner, self.repo, self.tag, self.filename
                )
            }
            Proxy::Xget => {
                format!(
                    "https://xget.xi-xu.me/gh/{}/{}/releases/download/{}/{}",
                    self.owner, self.repo, self.tag, self.filename
                )
            }
            Proxy::GhProxy => {
                format!(
                    "https://gh-proxy.com/https://github.com/{}/{}/releases/download/{}/{}",
                    self.owner, self.repo, self.tag, self.filename
                )
            }
            Proxy::Jsdelivr => {
                format!(
                    "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
                    self.owner, self.repo, self.tag, self.filename
                )
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum GitHubUrl {
    RawFile(RawFile),
    ReleaseAsset(ReleaseAsset),
}

impl GitHubUrl {
    pub fn parse(url: &str) -> Result<Self, ConversionError> {
        // Try parsing as raw file first
        if let Ok(raw_file) = RawFile::parse(url) {
            return Ok(GitHubUrl::RawFile(raw_file));
        }

        // Try parsing as release asset
        if let Ok(release_asset) = ReleaseAsset::parse(url) {
            return Ok(GitHubUrl::ReleaseAsset(release_asset));
        }

        // If both fail, return unsupported URL type error
        Err(ConversionError::UnsupportedUrlType(
            "URL does not match raw file or release asset patterns".to_string(),
        ))
    }

    pub fn url(&self, proxy: &Proxy) -> String {
        match self {
            GitHubUrl::RawFile(raw_file) => raw_file.url(proxy),
            GitHubUrl::ReleaseAsset(release_asset) => release_asset.url(proxy),
        }
    }
}

pub fn convert_url(proxy: &str, github_url: &str) -> Result<String, ConversionError> {
    // Parse proxy type
    let proxy = Proxy::from_str(proxy)?;

    // Parse GitHub URL
    let url = GitHubUrl::parse(github_url)?;

    // Generate and return proxied URL
    Ok(url.url(&proxy))
}
