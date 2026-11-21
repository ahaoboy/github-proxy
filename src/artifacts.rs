//! GitHub Actions / release artifact models and private-asset downloads.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use serde::Deserialize;

use crate::Resource;
use crate::fetch::{self, Client, Error, Options};

/// One downloadable asset in a release (or an Actions artifact).
#[derive(Eq, Deserialize, Debug, Clone)]
pub struct GhArtifact {
    /// Asset file name, e.g. `tool-x86_64-unknown-linux-musl.tar.gz`.
    pub name: String,
    /// Public download URL (`/releases/download/…`).
    pub browser_download_url: String,
    /// Size in bytes. `0` when unknown (e.g. parsed from release HTML).
    #[serde(default)]
    pub size: u64,
    /// API endpoint, present in single-release responses. Required to fetch a
    /// private asset.
    #[serde(default)]
    pub url: Option<String>,
}

// Equality and hashing deliberately use only `name`, so sets can be built from
// either source (API listing, single release, HTML fallback) without duplicates
// caused by differing URL forms.
impl PartialEq for GhArtifact {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for GhArtifact {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

/// Response of the GitHub releases API (`assets` subset).
#[derive(Debug, Default, Deserialize)]
pub struct GhArtifacts {
    /// Assets attached to the release.
    pub assets: HashSet<GhArtifact>,
}

/// Fetch a private release asset through the GitHub API.
///
/// A `browser_download_url` does not accept API tokens for private repos:
/// GitHub answers 404 to avoid revealing that the repository exists. The
/// supported route is the asset API endpoint with
/// `Accept: application/octet-stream`, which redirects to a pre-signed CDN URL.
///
/// Returns `Ok(None)` whenever the fallback does not apply — not a release
/// URL, no token, release or asset not found, or the API refused the request.
/// That lets callers keep their original error.
pub async fn private_asset(
    client: &Client,
    url: &str,
    opts: Options<'_>,
) -> Result<Option<Vec<u8>>, Error> {
    // The API is useless without a token: private assets are invisible to
    // anonymous requests.
    if opts.token.is_none_or(|token| token.is_empty()) {
        return Ok(None);
    }
    let Ok(Resource::Release {
        owner,
        repo,
        tag,
        name,
    }) = Resource::try_from(url)
    else {
        return Ok(None);
    };

    let release_api = format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}");
    let Ok(release) = fetch::json_with::<GhArtifacts>(client, &release_api, opts).await else {
        return Ok(None);
    };

    let Some(asset) = release
        .assets
        .iter()
        .find(|asset| asset.name == name || asset.browser_download_url == url)
    else {
        return Ok(None);
    };
    let Some(asset_api) = &asset.url else {
        return Ok(None);
    };

    // Token and User-Agent come from `fetch::get`; only the media type differs.
    let mut accept = HeaderMap::new();
    accept.insert(ACCEPT, HeaderValue::from_static("application/octet-stream"));

    let response = fetch::get(client, asset_api, opts, Some(accept)).await?;
    Ok(Some(
        response
            .bytes()
            .await
            .map_err(|source| Error::Body {
                url: asset_api.clone(),
                source,
            })?
            .to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifacts_are_unique_by_name() {
        let a = GhArtifact {
            name: "tool.zip".into(),
            browser_download_url: "https://github.com/o/r/releases/download/v1/tool.zip".into(),
            size: 10,
            url: None,
        };
        let b = GhArtifact {
            name: "tool.zip".into(),
            browser_download_url: "https://example.com/tool.zip".into(),
            size: 20,
            url: Some("https://api.github.com/x".into()),
        };
        let set: HashSet<_> = [a.clone(), b].into_iter().collect();
        assert_eq!(set.len(), 1);
        assert!(set.contains(&a));
    }
}
