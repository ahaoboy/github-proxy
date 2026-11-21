//! GitHub URL rewriting, plus optional HTTP fetching and a small CLI.

mod error;
mod proxy;
mod resource;

#[cfg(feature = "artifacts")]
pub mod artifacts;
#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "fetch")]
pub mod fetch;
#[cfg(feature = "token")]
pub mod token;

#[cfg(all(feature = "fetch", target_arch = "wasm32"))]
compile_error!("`github-proxy`: feature `fetch` needs a native target (reqwest/tokio)");

pub use error::ConversionError;
pub use proxy::Proxy;
pub use resource::Resource;

/// The `reqwest` version these helpers are built against.
///
/// Only needed when you pass your own client via the `*_with` functions or
/// [`fetch::builder`]; taking the type from here (instead of adding your own
/// `reqwest` dependency) guarantees a single version is linked.
#[cfg(feature = "fetch")]
pub use reqwest;
