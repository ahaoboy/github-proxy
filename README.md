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

#### Basic Syntax

```bash
github-proxy <proxy-type> <resource-type> <args...>
```

#### Proxy Types

- `github` - Native GitHub (no proxy)
- `gh-proxy` - gh-proxy.com service
- `xget` - xget.xi-xu.me service
- `jsdelivr` - cdn.jsdelivr.net service (files only, does not support release assets)
- `statically` - cdn.statically.io service (files only, does not support release assets)

#### Resource Types

**File Resources**

```bash
github-proxy <proxy-type> file <owner> <repo> <reference> <path>
```

- `owner` - Repository owner
- `repo` - Repository name
- `reference` - Branch name, tag, commit hash, or `refs/heads/branch`
- `path` - File path in the repository

**Release Resources**

```bash
github-proxy <proxy-type> release <owner> <repo> <tag> <name>
```

- `owner` - Repository owner
- `repo` - Repository name
- `tag` - Release tag
- `name` - Asset filename

#### Examples

**Convert a raw file URL:**

```bash
# Using xget proxy
github-proxy xget file easy-install easy-install main install.sh
# Output: https://xget.xi-xu.me/gh/easy-install/easy-install/raw/main/install.sh

# Using jsdelivr CDN
github-proxy jsdelivr file owner repo main src/lib.rs
# Output: https://cdn.jsdelivr.net/gh/owner/repo@main/src/lib.rs

# Using refs/heads format
github-proxy xget file owner repo refs/heads/main src/lib.rs
# Output: https://xget.xi-xu.me/gh/owner/repo/raw/refs/heads/main/src/lib.rs
```

**Convert a release asset URL:**

```bash
# Using gh-proxy
github-proxy gh-proxy release easy-install easy-install nightly ei-aarch64-apple-darwin.tar.gz
# Output: https://gh-proxy.com/https://github.com/easy-install/easy-install/releases/download/nightly/ei-aarch64-apple-darwin.tar.gz

# Using xget
github-proxy xget release fish-shell fish-shell 4.1.2 fish-4.1.2-linux-aarch64.tar.xz
# Output: https://xget.xi-xu.me/gh/fish-shell/fish-shell/releases/download/4.1.2/fish-4.1.2-linux-aarch64.tar.xz
```

### Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
github-proxy = "0.1"
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
- `wasm` - Enable WebAssembly support with wasm-bindgen

```toml
[dependencies]
github-proxy = { version = "0.1", features = ["serde"] }
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