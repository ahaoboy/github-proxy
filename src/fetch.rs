//! Fetching GitHub resources over HTTP.
//!
//! Most callers need one thing: bytes/text/JSON from a URL. The primary API
//! therefore takes no `reqwest` types at all and uses a shared, preconfigured
//! client, so `reqwest` stays an implementation detail that can be upgraded
//! without breaking consumers.
//!
//! The client is configured for what GitHub specifically needs:
//!
//! * rustls with the bundled Mozilla roots — Android/Termux exposes no system
//!   trust store to native processes, and `rustls-platform-verifier` needs a
//!   JNI-initialised Android context there.
//! * HTTP/1.1 only, avoiding ALPN negotiation on mobile networks.
//! * IPv4-first DNS with a bounded lookup: mobile networks often advertise an
//!   unroutable IPv6 address, and `getaddrinfo` can block for a long time on
//!   Android when no nameserver is reachable.
//! * a bearer token sent only to GitHub hosts.
//!
//! Every function performs exactly one request; retry/backoff belongs to the
//! caller.
//!
//! Callers that need their own client (cookies, proxies, a different timeout
//! policy) can use [`builder`] plus the `*_with` variants; those signatures do
//! mention `reqwest`, which is why the crate re-exports it.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use reqwest::Url as ReqwestUrl;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
pub use reqwest::{Client, ClientBuilder, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;

/// Upper bound for a single DNS lookup.
const DNS_TIMEOUT: Duration = Duration::from_secs(10);

/// `User-Agent` sent unless the caller overrides it.
const USER_AGENT_VALUE: &str = "reqwest";

/// Anything that can go wrong while fetching a resource.
#[derive(Debug, Error)]
pub enum Error {
    /// The URL could not be parsed.
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl {
        /// The offending input.
        url: String,
        /// Why parsing failed.
        reason: String,
    },

    /// Only http/https are accepted.
    #[error("unsupported URL scheme '{scheme}' (only http and https): {url}")]
    UnsupportedScheme {
        /// The offending input.
        url: String,
        /// The parsed scheme.
        scheme: String,
    },

    /// A header value could not be built (e.g. a token with invalid bytes).
    #[error("invalid HTTP header value: {0}")]
    InvalidHeader(String),

    /// The request exceeded its timeout.
    #[error("request to {url} timed out after {secs}s")]
    Timeout {
        /// The requested URL.
        url: String,
        /// The configured timeout.
        secs: u64,
    },

    /// The request could not be sent (DNS, TCP, TLS, …).
    #[error("request to {url} failed")]
    Request {
        /// The requested URL.
        url: String,
        /// The underlying transport error.
        #[source]
        source: reqwest::Error,
    },

    /// The server returned a non-success status.
    #[error("request to {url} returned HTTP {status}")]
    Status {
        /// The requested URL.
        url: String,
        /// The response status.
        status: StatusCode,
    },

    /// The response body could not be read.
    #[error("failed to read the response body from {url}")]
    Body {
        /// The requested URL.
        url: String,
        /// The underlying transport error.
        #[source]
        source: reqwest::Error,
    },

    /// The response body could not be decoded.
    #[error("failed to decode the response from {url}")]
    Decode {
        /// The requested URL.
        url: String,
        /// The decoding error.
        #[source]
        source: reqwest::Error,
    },
}

impl Error {
    /// The HTTP status, when the failure was an unexpected response code.
    ///
    /// Callers use this to decide whether a fallback (e.g. fetching a private
    /// release asset through the API) is worth trying.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Error::Status { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Whether the request timed out.
    pub fn is_timeout(&self) -> bool {
        match self {
            Error::Timeout { .. } => true,
            Error::Request { source, .. } => source.is_timeout(),
            _ => false,
        }
    }
}

/// Per-request settings.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options<'a> {
    /// Bearer token. Only ever sent to GitHub hosts.
    pub token: Option<&'a str>,
    /// Per-request timeout. `None` keeps the client's default.
    pub timeout: Option<Duration>,
}

impl Options<'_> {
    /// No token, no per-request timeout.
    pub const fn new() -> Self {
        Self {
            token: None,
            timeout: None,
        }
    }
}

/// A URL validated as http(s).
///
/// Constructing one is the only way to enable a request, so scheme and syntax
/// checks cannot be skipped. Host classification lives here too, keeping these
/// rules out of the public function namespace and free to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url(ReqwestUrl);

impl Url {
    /// Parse `input`, accepting only `http` and `https`.
    pub fn parse(input: &str) -> Result<Self, Error> {
        let parsed = ReqwestUrl::parse(input).map_err(|e| Error::InvalidUrl {
            url: input.to_string(),
            reason: e.to_string(),
        })?;
        match parsed.scheme() {
            "http" | "https" => Ok(Self(parsed)),
            scheme => Err(Error::UnsupportedScheme {
                url: input.to_string(),
                scheme: scheme.to_string(),
            }),
        }
    }

    /// Whether `input` is a valid http(s) URL.
    pub fn is_valid(input: &str) -> bool {
        Self::parse(input).is_ok()
    }

    /// The URL as written.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Whether requests to this URL should carry the GitHub token.
    ///
    /// Covers `github.com`, its subdomains (including `api.github.com`) and
    /// `githubusercontent.com`.
    pub fn is_github(&self) -> bool {
        matches!(self.0.host_str(), Some(host)
            if host == "github.com"
                || host.ends_with(".github.com")
                || host == "githubusercontent.com"
                || host.ends_with(".githubusercontent.com"))
    }

    /// Request headers for this URL.
    ///
    /// The token is attached only for GitHub hosts, so it cannot leak to an
    /// unrelated CDN that a proxy redirected to.
    pub fn headers(&self, token: Option<&str>) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));

        if let Some(token) = token.filter(|token| !token.is_empty() && self.is_github()) {
            let value = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|e| Error::InvalidHeader(e.to_string()))?;
            headers.insert(AUTHORIZATION, value);
        }

        Ok(headers)
    }
}

impl std::str::FromStr for Url {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input)
    }
}

/// A [`ClientBuilder`] preconfigured for GitHub traffic.
pub fn builder() -> ClientBuilder {
    Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(true)
        .http1_only()
        .dns_resolver(Arc::new(PreferIpv4))
}

/// The shared client used by [`bytes`], [`text`] and [`json`].
fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| builder().build().expect("failed to build the HTTP client"))
}

/// One GET; returns the response only on a 2xx status.
pub(crate) async fn get(
    client: &Client,
    url: &str,
    opts: Options<'_>,
    extra: Option<HeaderMap>,
) -> Result<reqwest::Response, Error> {
    let parsed = Url::parse(url)?;
    let mut request = client
        .get(parsed.0.clone())
        .headers(parsed.headers(opts.token)?);
    if let Some(timeout) = opts.timeout {
        request = request.timeout(timeout);
    }
    if let Some(extra) = extra {
        request = request.headers(extra);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(e) if e.is_timeout() => {
            return Err(Error::Timeout {
                url: url.to_string(),
                secs: opts.timeout.map(|t| t.as_secs()).unwrap_or_default(),
            });
        }
        Err(e) => {
            return Err(Error::Request {
                url: url.to_string(),
                source: e,
            });
        }
    };

    let status = response.status();
    if !status.is_success() {
        return Err(Error::Status {
            url: url.to_string(),
            status,
        });
    }
    Ok(response)
}

/// Fetch `url` as bytes with the shared client.
pub async fn bytes(url: &str, opts: Options<'_>) -> Result<Vec<u8>, Error> {
    bytes_with(client(), url, opts).await
}

/// Fetch `url` as text with the shared client.
pub async fn text(url: &str, opts: Options<'_>) -> Result<String, Error> {
    text_with(client(), url, opts).await
}

/// Fetch `url` as JSON with the shared client.
pub async fn json<T: DeserializeOwned>(url: &str, opts: Options<'_>) -> Result<T, Error> {
    json_with(client(), url, opts).await
}

/// Fetch `url` as bytes with a caller-provided client.
pub async fn bytes_with(client: &Client, url: &str, opts: Options<'_>) -> Result<Vec<u8>, Error> {
    get(client, url, opts, None)
        .await?
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|source| Error::Body {
            url: url.to_string(),
            source,
        })
}

/// Fetch `url` as text with a caller-provided client.
pub async fn text_with(client: &Client, url: &str, opts: Options<'_>) -> Result<String, Error> {
    get(client, url, opts, None)
        .await?
        .text()
        .await
        .map_err(|source| Error::Body {
            url: url.to_string(),
            source,
        })
}

/// Fetch `url` as JSON with a caller-provided client.
pub async fn json_with<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    opts: Options<'_>,
) -> Result<T, Error> {
    get(client, url, opts, None)
        .await?
        .json::<T>()
        .await
        .map_err(|source| Error::Decode {
            url: url.to_string(),
            source,
        })
}

/// Resolver that returns IPv4 addresses before IPv6 ones.
///
/// `reqwest`/hyper try the resolved addresses **in order**, and one connect
/// timeout covers DNS + TCP + TLS. On mobile networks an advertised but
/// unroutable IPv6 address is common: it silently drops packets, consumes the
/// whole timeout, and the request fails even though IPv4 works — which is why
/// `curl` (Happy Eyeballs) succeeds where a plain client fails.
struct PreferIpv4;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl Resolve for PreferIpv4 {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            // Numeric hosts need no lookup. Port 0 is replaced by reqwest with
            // the scheme's default port.
            if let Ok(ip) = host.parse::<IpAddr>() {
                return Ok(Box::new(std::iter::once(SocketAddr::new(ip, 0))) as Addrs);
            }

            if dns_unavailable(&host) {
                return Err::<Addrs, BoxError>(android_dns_error(&host).into());
            }

            let lookup = tokio::net::lookup_host((host.as_str(), 0));
            let collected: Vec<SocketAddr> = match tokio::time::timeout(DNS_TIMEOUT, lookup).await {
                Err(_) => {
                    return Err::<Addrs, BoxError>(
                        format!("DNS lookup timed out for {host}").into(),
                    );
                }
                Ok(Err(e)) => {
                    return Err::<Addrs, BoxError>(
                        format!("DNS lookup failed for {host}: {e}").into(),
                    );
                }
                Ok(Ok(addrs)) => addrs.collect(),
            };

            if collected.is_empty() {
                return Err::<Addrs, BoxError>(
                    format!("DNS lookup returned no addresses for {host}").into(),
                );
            }

            let (mut ipv4, ipv6): (Vec<_>, Vec<_>) =
                collected.into_iter().partition(SocketAddr::is_ipv4);
            ipv4.extend(ipv6);
            Ok(Box::new(ipv4.into_iter()) as Addrs)
        })
    }
}

/// True when this process runs on Android, even if it was not compiled for it.
///
/// A `*-unknown-linux-musl` build reports `target_os = "linux"` but may still
/// run inside Termux or another Android rootfs.
fn running_on_android() -> bool {
    cfg!(target_os = "android")
        || std::env::var_os("ANDROID_ROOT").is_some()
        || std::path::Path::new("/system/build.prop").exists()
}

/// Whether a DNS lookup for `host` is guaranteed to fail on this host.
///
/// Android exposes DNS only through its native resolver service (`netd`).
/// Bionic builds talk to it; static musl/glibc builds cannot, and Android has
/// no `/etc/resolv.conf`. There `getaddrinfo()` does not fail — it blocks,
/// which users see as "the download hangs forever". Failing fast turns that
/// into an actionable error.
fn dns_unavailable(host: &str) -> bool {
    dns_unavailable_with(
        running_on_android(),
        cfg!(target_os = "android"),
        host,
        std::fs::read_to_string("/etc/hosts").ok().as_deref(),
        std::fs::read_to_string("/etc/resolv.conf").ok().as_deref(),
    )
}

/// Pure decision logic for [`dns_unavailable`], so it can be tested.
fn dns_unavailable_with(
    on_android: bool,
    bionic_build: bool,
    host: &str,
    hosts_file: Option<&str>,
    resolv_conf: Option<&str>,
) -> bool {
    if !on_android || bionic_build {
        return false;
    }
    if hosts_file.is_some_and(|content| host_in_hosts(content, host)) {
        return false;
    }
    !resolv_conf.is_some_and(resolv_conf_has_nameserver)
}

/// Parse a `resolv.conf` body the way musl/glibc do: a usable `nameserver`.
fn resolv_conf_has_nameserver(content: &str) -> bool {
    content.lines().any(|line| {
        let mut fields = line.split_whitespace();
        matches!(fields.next(), Some("nameserver"))
            && fields
                .next()
                .is_some_and(|ns| !ns.is_empty() && ns != "0.0.0.0" && ns != "::")
    })
}

/// Whether `host` is mapped in an `/etc/hosts` body.
fn host_in_hosts(content: &str, host: &str) -> bool {
    content
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .any(|line| {
            line.split_whitespace()
                .skip(1)
                .any(|name| name.eq_ignore_ascii_case(host))
        })
}

/// Actionable message for [`dns_unavailable`].
fn android_dns_error(host: &str) -> String {
    format!(
        "cannot resolve '{host}': this build has no working DNS on Android

  Static musl/glibc builds cannot reach Android's native resolver (`netd`),
  and /etc/resolv.conf does not exist here. getaddrinfo() blocks instead of
  failing, so downloads appear to hang.

  Use a build linked against bionic (e.g. aarch64-linux-android),
  map '{host}' in /etc/hosts, or pass a direct artifact URL."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn validates_schemes() {
        assert!(Url::parse("https://github.com/o/r").is_ok());
        assert!(Url::parse("http://github.com/o/r").is_ok());
        assert!(matches!(
            Url::parse("ftp://github.com/o/r"),
            Err(Error::UnsupportedScheme { .. })
        ));
        assert!(matches!(
            Url::parse("not a url"),
            Err(Error::InvalidUrl { .. })
        ));
        assert!(Url::is_valid("https://github.com/o/r"));
        assert!(!Url::is_valid(""));
    }

    #[test]
    fn identifies_github_hosts() {
        for host in [
            "https://github.com/o/r",
            "https://api.github.com/repos/o/r",
            "https://raw.githubusercontent.com/o/r/main/x",
            "https://objects.githubusercontent.com/x",
        ] {
            assert!(url(host).is_github(), "{host}");
        }
        for host in ["https://example.com/x", "https://notgithub.com/x"] {
            assert!(!url(host).is_github(), "{host}");
        }
    }

    #[test]
    fn token_only_goes_to_github() {
        let with_token = url("https://github.com/o/r").headers(Some("t")).unwrap();
        assert_eq!(with_token.get(AUTHORIZATION).unwrap(), "Bearer t");

        let other = url("https://cdn.example.com/x").headers(Some("t")).unwrap();
        assert!(other.get(AUTHORIZATION).is_none());

        let empty = url("https://github.com/o/r").headers(Some("")).unwrap();
        assert!(empty.get(AUTHORIZATION).is_none());
    }

    #[test]
    fn resolv_conf_requires_a_usable_nameserver() {
        assert!(resolv_conf_has_nameserver("nameserver 192.168.1.1\n"));
        assert!(!resolv_conf_has_nameserver("#nameserver 8.8.8.8\n"));
        assert!(!resolv_conf_has_nameserver(""));
        assert!(!resolv_conf_has_nameserver("nameserver 0.0.0.0\n"));
        assert!(!resolv_conf_has_nameserver("nameserver ::"));
    }

    #[test]
    fn hosts_lookup_matches_names_not_addresses() {
        let hosts = "127.0.0.1 localhost\n10.0.0.5 api.internal # note\n";
        assert!(host_in_hosts(hosts, "api.internal"));
        assert!(host_in_hosts(hosts, "LOCALHOST"));
        assert!(!host_in_hosts(hosts, "10.0.0.5"));
        assert!(!host_in_hosts(hosts, "note"));
    }

    #[test]
    fn android_without_resolver_fails_fast() {
        assert!(dns_unavailable_with(
            true,
            false,
            "github.com",
            Some("127.0.0.1 localhost\n"),
            None
        ));
        assert!(!dns_unavailable_with(true, true, "github.com", None, None));
        assert!(!dns_unavailable_with(
            false,
            false,
            "github.com",
            None,
            None
        ));
        assert!(!dns_unavailable_with(
            true,
            false,
            "github.com",
            None,
            Some("nameserver 8.8.8.8\n")
        ));
        assert!(!dns_unavailable_with(
            true,
            false,
            "github.com",
            Some("140.82.1.1 github.com\n"),
            None
        ));
    }
}
