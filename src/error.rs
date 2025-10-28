use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Invalid proxy type: {0}\nSupported types: github, gh-proxy, xget, jsdelivr")]
    InvalidProxyType(String),

    #[error("Invalid resource type: {0}\nSupported types: file, release")]
    InvalidResourceType(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
}
