use github_proxy::convert_url;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: github-proxy <proxy-type> <github-url>");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  <proxy-type>   Proxy service: github, gh-proxy, xget, jsdelivr");
        eprintln!("  <github-url>   GitHub URL to convert");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  github-proxy xget https://github.com/owner/repo/raw/main/file.sh");
        eprintln!(
            "  github-proxy gh-proxy https://github.com/owner/repo/releases/download/v1.0/file.tar.gz"
        );
        std::process::exit(1);
    }

    let proxy_type = &args[1];
    let github_url = &args[2];

    match convert_url(proxy_type, github_url) {
        Ok(proxied_url) => {
            println!("{}", proxied_url);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
