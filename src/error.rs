use thiserror::Error;

/// Failure to convert between URLs, mirror names and resources.
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Unknown mirror name.
    #[error("invalid proxy '{0}'\nsupported: github, gh-proxy, xget, jsdelivr, statically")]
    InvalidProxyType(String),

    /// The input is not a supported GitHub URL.
    #[error(
        "invalid GitHub URL: {0}\nsupported:\n  https://github.com/<owner>/<repo>/raw/<ref>/<path>\n  https://github.com/<owner>/<repo>/blob/<ref>/<path>\n  https://github.com/<owner>/<repo>/releases/download/<tag>/<name>"
    )]
    InvalidUrl(String),

    /// The URL matched but its reference/path could not be split.
    #[error("cannot parse reference/path: {0}")]
    ParseError(String),
}
