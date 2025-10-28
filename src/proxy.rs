use crate::{GitHubResource, error::ConversionError};
use std::{fmt, str::FromStr};
use strum_macros::EnumIter;

/// Proxy service types
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(EnumIter, Debug, PartialEq, Hash, Eq, Clone)]
pub enum Proxy {
    /// Native GitHub (no proxy)
    GitHub,
    /// gh-proxy.com service
    GhProxy,
    /// xget.xi-xu.me service
    Xget,
    /// cdn.jsdelivr.net service
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

impl Proxy {
    pub fn url(&self, resource: GitHubResource) -> Option<String> {
        resource.url(self)
    }
}
