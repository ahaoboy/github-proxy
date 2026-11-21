use std::{fmt, str::FromStr};

use strum_macros::EnumIter;

use crate::{Resource, error::ConversionError};

/// GitHub download mirrors.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
#[derive(EnumIter, Debug, PartialEq, Hash, Eq, Clone, Copy, Default)]
pub enum Proxy {
    /// Native GitHub, no mirror.
    #[default]
    Github,
    /// `gh-proxy.com`.
    GhProxy,
    /// `xget.xi-xu.me`.
    Xget,
    /// `cdn.jsdelivr.net` — repository files only.
    Jsdelivr,
    /// `cdn.statically.io` — repository files only.
    Statically,
}

impl Proxy {
    /// Every supported mirror.
    pub const ALL: [Proxy; 5] = [
        Proxy::Github,
        Proxy::GhProxy,
        Proxy::Xget,
        Proxy::Jsdelivr,
        Proxy::Statically,
    ];

    /// Canonical lower-case name (`github`, `gh-proxy`, …).
    pub const fn as_str(self) -> &'static str {
        match self {
            Proxy::Github => "github",
            Proxy::GhProxy => "gh-proxy",
            Proxy::Xget => "xget",
            Proxy::Jsdelivr => "jsdelivr",
            Proxy::Statically => "statically",
        }
    }

    /// Whether this mirror can serve release assets.
    ///
    /// jsDelivr and Statically only reach repository files, so
    /// `/releases/download/` URLs cannot be served.
    pub const fn supports_release(self) -> bool {
        matches!(self, Proxy::Github | Proxy::GhProxy | Proxy::Xget)
    }

    /// Proxied URL for `resource`, or `None` when the mirror cannot serve it.
    pub fn url(self, resource: &Resource) -> Option<String> {
        resource.url(&self)
    }
}

impl fmt::Display for Proxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Proxy {
    type Err = ConversionError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|proxy| proxy.as_str().eq_ignore_ascii_case(input))
            .ok_or_else(|| ConversionError::InvalidProxyType(input.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_names() {
        for proxy in Proxy::ALL {
            assert_eq!(Proxy::from_str(proxy.as_str()).unwrap(), proxy);
            assert_eq!(proxy.to_string(), proxy.as_str());
        }
        assert_eq!(Proxy::from_str("XGET").unwrap(), Proxy::Xget);
        assert!(Proxy::from_str("ftp").is_err());
    }

    #[test]
    fn mirrors_advertise_release_support() {
        assert!(Proxy::Github.supports_release());
        assert!(Proxy::GhProxy.supports_release());
        assert!(Proxy::Xget.supports_release());
        assert!(!Proxy::Jsdelivr.supports_release());
        assert!(!Proxy::Statically.supports_release());
    }
}
