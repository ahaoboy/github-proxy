use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error(
        "Invalid proxy type: {0}\nSupported types: github, gh-proxy, xget, jsdelivr, statically"
    )]
    InvalidProxyType(String),

    #[error("Invalid resource type: {0}\nSupported types: file, release")]
    InvalidResourceType(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error(
        "Invalid GitHub URL: {0}\nSupported formats:\n  - https://github.com/{{owner}}/{{repo}}/raw/{{ref}}/{{path}}\n  - https://github.com/{{owner}}/{{repo}}/blob/{{ref}}/{{path}}\n  - https://github.com/{{owner}}/{{repo}}/releases/download/{{tag}}/{{filename}}"
    )]
    InvalidUrl(String),

    #[error("URL parse error: {0}")]
    ParseError(String),
}
