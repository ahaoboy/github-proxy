//! GitHub token detection.
//!
//! The token is optional: public downloads work without one, and a private
//! repository needs it to reach the API. Detection is best-effort and cached
//! for the process, because the `gh`/`git` probes spawn subprocesses.

use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Upper bound for each credential probe.
pub const DETECT_TIMEOUT: Duration = Duration::from_secs(5);

static CACHE: OnceLock<Option<String>> = OnceLock::new();

/// Detect a GitHub token, if any.
///
/// Tried in order: `GITHUB_TOKEN`, `gh auth token`, `git credential fill`.
/// The result (including "none") is cached for the process lifetime.
pub async fn detect() -> Option<String> {
    if let Some(cached) = CACHE.get() {
        return cached.clone();
    }

    let token = match from_env() {
        Some(token) => Some(token),
        None => match github_cli().await {
            Some(token) => Some(token),
            None => git_credential().await,
        },
    };

    let _ = CACHE.set(token.clone());
    token
}

/// `GITHUB_TOKEN` from the environment.
///
/// Checked first: it is the most reliable source in CI and needs no
/// subprocess.
pub fn from_env() -> Option<String> {
    std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
}

/// `gh auth token` (via PowerShell on Windows, where `gh` is a shim).
async fn github_cli() -> Option<String> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("powershell");
        cmd.args(["-c", "gh auth token"]);
        cmd
    };

    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut cmd = Command::new("gh");
        cmd.args(["auth", "token"]);
        cmd
    };

    let output = tokio::time::timeout(DETECT_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// `git credential fill` for `https://github.com`.
async fn git_credential() -> Option<String> {
    let mut child = Command::new("git")
        .args(["credential", "fill"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .spawn()
        .ok()?;

    let mut stdin = child.stdin.take()?;
    let input = "protocol=https\nhost=github.com\n\n";
    tokio::time::timeout(DETECT_TIMEOUT, stdin.write_all(input.as_bytes()))
        .await
        .ok()?
        .ok()?;
    drop(stdin); // signal EOF so `git credential` answers

    let output = tokio::time::timeout(DETECT_TIMEOUT, child.wait_with_output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    credential_field(&text, "password").filter(|token| !token.is_empty())
}

/// Read `key=value` from `git credential` output.
fn credential_field(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        (name.trim() == key).then(|| value.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_credential_output() {
        let output = "protocol=https\nhost=github.com\nusername=u\npassword=tok\n\n";
        assert_eq!(credential_field(output, "password").as_deref(), Some("tok"));
        assert_eq!(
            credential_field(output, "host").as_deref(),
            Some("github.com")
        );
        assert_eq!(credential_field(output, "missing"), None);
        assert_eq!(credential_field("garbage", "password"), None);
    }
}
