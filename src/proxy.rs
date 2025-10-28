use crate::error::ConversionError;
use std::{fmt, str::FromStr};

/// Proxy service types
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ProxyType {
    /// Native GitHub (no proxy)
    GitHub,
    /// gh-proxy.com service
    GhProxy,
    /// xget.xi-xu.me service
    Xget,
    /// cdn.jsdelivr.net service
    Jsdelivr,
}

impl FromStr for ProxyType {
    type Err = ConversionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(ProxyType::GitHub),
            "gh-proxy" => Ok(ProxyType::GhProxy),
            "xget" => Ok(ProxyType::Xget),
            "jsdelivr" => Ok(ProxyType::Jsdelivr),
            _ => Err(ConversionError::InvalidProxyType(s.to_string())),
        }
    }
}

impl fmt::Display for ProxyType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProxyType::GitHub => write!(f, "github"),
            ProxyType::GhProxy => write!(f, "gh-proxy"),
            ProxyType::Xget => write!(f, "xget"),
            ProxyType::Jsdelivr => write!(f, "jsdelivr"),
        }
    }
}
