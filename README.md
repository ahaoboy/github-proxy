# github-proxy

A Rust CLI tool and library for converting GitHub URLs to use various proxy services, enabling faster downloads and better accessibility in restricted regions.

## Features

- 🚀 Convert GitHub raw file URLs to proxy services
- 📦 Convert GitHub release asset URLs to proxy services
- 🔧 Use as a CLI tool or Rust library
- 🌐 Support for multiple proxy services:
  - [jsdelivr](https://www.jsdelivr.com/github) - CDN for files only
  - [gh-proxy](https://gh-proxy.com/) - Full proxy support
  - [xget](https://xuc.xi-xu.me/) - Full proxy support
  - [statically](https://statically.io/convert/) - CDN for files only
  - Native GitHub (no proxy)

## Installation

### From crates.io

```bash
cargo install github-proxy
```

### From source

```bash
git clone https://github.com/ahaoboy/github-proxy
cd github-proxy
cargo install --path .
```

## Usage

### CLI

Enabled with the `cli` feature:

```bash
cargo install github-proxy --features cli
```

Two subcommands: `ls` to query, `fetch` to download.

```
Usage: github-proxy [OPTIONS] <COMMAND>

Commands:
  ls     List mirrors, or the assets of a release
  fetch  Download a resource

Options:
  -p, --proxy <PROXY>  Mirror to use [default: github]
                       [possible values: github, gh-proxy, xget, jsdelivr, statically]
```

#### `ls` — query

```bash
# Supported mirrors and what they can serve
github-proxy ls

# Assets of the latest release: size and name
github-proxy ls quickjs-ng/quickjs

# Assets of a specific tag
github-proxy ls quickjs-ng/quickjs@v0.18.0

# Only one asset
github-proxy ls quickjs-ng/quickjs@v0.18.0 --name qjs-linux-aarch64

# Download URLs instead of size/name
github-proxy -p xget ls quickjs-ng/quickjs@v0.18.0 --url
```

```
$ github-proxy ls quickjs-ng/quickjs@v0.18.0
  3.8 MB  qjs-cosmo.com
  1.3 MB  qjs-darwin-arm64
  2.4 MB  qjs-linux-aarch64
  ...
```

`jsdelivr` and `statically` only mirror repository files, so listing release
assets through them fails with an explanation rather than printing unusable
URLs.

#### `fetch` — download

```
Usage: github-proxy fetch [OPTIONS] <SPEC> [NAME]
```

`SPEC` is a GitHub URL or `owner/repo[@tag][/name]`. Omitting the tag uses the
repository's **latest release**. `NAME` is optional and, when given, means
**everything after `@` is the tag** — no guessing.

```bash
# Latest release
github-proxy fetch quickjs-ng/quickjs qjs-linux-aarch64

# Name embedded in the spec (last segment)
github-proxy fetch quickjs-ng/quickjs@v0.18.0/qjs-linux-aarch64

# Separate name: unambiguous even when the tag contains `/`
github-proxy fetch quickjs-ng/quickjs@release/1.0 qjs-linux-aarch64

# Explicit destination, or an existing directory
github-proxy fetch quickjs-ng/quickjs@v0.18.0 qjs-linux-aarch64 -o /opt/qjs
github-proxy fetch quickjs-ng/quickjs@v0.18.0 qjs-linux-aarch64 -o ./bin/

# Raw files work too, by URL (the proxy still applies)
github-proxy -p xget fetch https://github.com/quickjs-ng/quickjs/raw/master/README.md
```

Without `-o` the asset is written to its own name in the current directory.

> **Tags containing `/`:** git allows tags like `release/1.0`, so
> `owner/repo@release/1.0/a.zip` is ambiguous. Pass the name as the second
> positional (`fetch owner/repo@release/1.0 a.zip`) and the tag is taken
> verbatim.

A `GITHUB_TOKEN` (or `gh auth login` / `git credential`) is picked up
automatically, so private repositories work too.

### Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
github-proxy = "0.2"
```

#### Basic Example

```rust
use github_proxy::{Resource, Proxy};
use std::str::FromStr;

fn main() {
    // Create a file resource
    let resource = Resource::file(
        "owner".to_string(),
        "repo".to_string(),
        "main".to_string(),
        "install.sh".to_string(),
    );

    // Convert to xget proxy URL
    let proxy = Proxy::Xget;
    if let Some(url) = resource.url(&proxy) {
        println!("{}", url);
        // Output: https://xget.xi-xu.me/gh/owner/repo/raw/main/install.sh
    }

    // Create a release resource
    let release = Resource::release(
        "owner".to_string(),
        "repo".to_string(),
        "v1.0.0".to_string(),
        "app.tar.gz".to_string(),
    );

    // Convert to gh-proxy URL
    let proxy = Proxy::GhProxy;
    if let Some(url) = release.url(&proxy) {
        println!("{}", url);
        // Output: https://gh-proxy.com/https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz
    }
}
```

#### Parse GitHub URLs

```rust
use github_proxy::Resource;

fn main() {
    // Parse a raw file URL
    let url = "https://github.com/owner/repo/raw/main/install.sh";
    let resource = Resource::try_from(url).unwrap();

    // Parse a blob file URL
    let url = "https://github.com/owner/repo/blob/main/src/lib.rs";
    let resource = Resource::try_from(url).unwrap();

    // Parse a release download URL
    let url = "https://github.com/owner/repo/releases/download/v1.0.0/app.tar.gz";
    let resource = Resource::try_from(url).unwrap();
}
```

#### Using Proxy Types

```rust
use github_proxy::Proxy;
use std::str::FromStr;

fn main() {
    // Parse proxy type from string
    let proxy = Proxy::from_str("xget").unwrap();

    // Use proxy enum directly
    let proxy = Proxy::Jsdelivr;

    println!("{}", proxy); // Output: jsdelivr
}
```

## Features

### Optional Features

- `serde` - Enable serde serialization support
- `fetch` - Shared HTTP client plus `bytes` / `text` / `json` helpers
- `token` - Best-effort `GITHUB_TOKEN` / `gh` / `git credential` detection
- `artifacts` - `GhArtifact(s)` models and private release asset downloads
- `cli` - The `github-proxy` command (implies `fetch`, `token`, `artifacts`)

```toml
[dependencies]
# URL/proxy helpers only
github-proxy = { version = "0.2", features = ["serde"] }

# Download GitHub content
github-proxy = { version = "0.2", features = ["fetch", "token", "artifacts"] }
```

### Downloading

The common API mentions no `reqwest` type, so updating `reqwest` inside this
crate is not a breaking change for you:

```rust,ignore
use github_proxy::fetch;
use github_proxy::token;

let token = token::detect().await;
let opts = fetch::Options {
    token: token.as_deref(),
    timeout: Some(std::time::Duration::from_secs(30)),
};

let bytes = fetch::bytes("https://github.com/o/r/releases/download/v1/a.zip", opts).await?;
let json  = fetch::json::<MyType>("https://api.github.com/repos/o/r/releases", opts).await?;
```

Every call is a single request; retry/backoff policy belongs to the application.

### Custom client

Need cookies, a proxy or a different pooling policy? Build your own client and
use the `*_with` variants. Take the `reqwest` types from this crate's re-export
so exactly one version is linked:

```rust,ignore
use github_proxy::fetch::{self, Client, Options};
use github_proxy::reqwest; // re-exported, same version as the crate

let client: Client = fetch::builder().cookie_store(true).build()?;
let bytes = fetch::bytes_with(&client, url, Options::new()).await?;

// Or configure from scratch:
let client = reqwest::Client::builder()
    .dns_resolver(/* ... */)
    .build()?;
```

> Only the `*_with` / `builder` / `Client` surface names `reqwest`. If you never
> use those, `reqwest` is a private dependency and may be upgraded in a minor
> release.

### URL checks

URL validation and GitHub-host classification live on the `Url` type, so they
cannot be called without a validated value:

```rust,ignore
use github_proxy::fetch::Url;

let url = Url::parse("https://github.com/o/r/releases/download/v1/a.zip")?;
assert!(url.is_github());

let headers = url.headers(Some(token)); // token only for GitHub hosts
assert!(Url::is_valid("https://api.github.com/repos/o/r"));
```

## Limitations

- **jsdelivr** and **statically** do not support release assets from `/releases/download/` paths
- Only GitHub URLs are supported for parsing

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT

## Repository

https://github.com/ahaoboy/github-proxy