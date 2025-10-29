use crate::error::ConversionError;
use crate::proxy::Proxy;
use crate::resource::Resource;
use std::str::FromStr as _;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }

    let proxy_type_str = &args[1];
    let resource_type_str = &args[2];

    // Parse proxy type
    let proxy_type = Proxy::from_str(proxy_type_str)?;

    // Parse resource based on type
    let resource = match resource_type_str.to_lowercase().as_str() {
        "file" => {
            if args.len() != 7 {
                return Err(ConversionError::InvalidArguments(
                    "file requires 4 arguments: owner repo reference path".to_string(),
                )
                .into());
            }
            Resource::file(
                args[3].clone(),
                args[4].clone(),
                args[5].clone(),
                args[6].clone(),
            )
        }
        "release" => {
            if args.len() != 7 {
                return Err(ConversionError::InvalidArguments(
                    "release requires 4 arguments: owner repo tag name".to_string(),
                )
                .into());
            }
            Resource::release(
                args[3].clone(),
                args[4].clone(),
                args[5].clone(),
                args[6].clone(),
            )
        }
        _ => {
            return Err(ConversionError::InvalidResourceType(resource_type_str.clone()).into());
        }
    };

    // Generate and print URL
    match resource.url(&proxy_type) {
        Some(url) => {
            println!("{}", url);
            Ok(())
        }
        None => {
            eprintln!(
                "Error: {} proxy does not support {:?} resources",
                proxy_type, resource
            );
            eprintln!(
                "Note: jsdelivr and statically do not support release assets from /releases/download/"
            );
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("Usage: github-proxy <proxy-type> <resource-type> <args...>");
    eprintln!();
    eprintln!("Proxy Types:");
    eprintln!("  github      Native Github (no proxy)");
    eprintln!("  gh-proxy    gh-proxy.com service");
    eprintln!("  xget        xget.xi-xu.me service");
    eprintln!("  jsdelivr    cdn.jsdelivr.net service (files only)");
    eprintln!("  statically  cdn.statically.io service (files only)");
    eprintln!();
    eprintln!("Resource Types:");
    eprintln!("  file <owner> <repo> <reference> <path>");
    eprintln!("    Generate URL for a raw file in repository");
    eprintln!("    reference can be: branch, tag, commit hash, or refs/heads/branch");
    eprintln!();
    eprintln!("  release <owner> <repo> <tag> <name>");
    eprintln!("    Generate URL for a release asset");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  github-proxy xget file easy-install easy-install main install.sh");
    eprintln!("  github-proxy xget file owner repo refs/heads/main src/lib.rs");
    eprintln!(
        "  github-proxy gh-proxy release easy-install easy-install nightly ei-aarch64-apple-darwin.tar.gz"
    );
}
