use sha2::{Digest, Sha256};
use std::io::Write;
use std::time::Duration;

const REPO: &str = "doublewordai/dw";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build a shared HTTP client with timeouts for update operations.
fn http_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("dw-cli")
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(10))
        .build()?)
}

/// Format an HTTP error with status code (or "network error" if no status).
fn format_http_error(e: &reqwest::Error) -> String {
    match e.status() {
        Some(status) => format!(
            "{} {}",
            status.as_u16(),
            status.canonical_reason().unwrap_or("Unknown")
        ),
        None => "network error".to_string(),
    }
}

/// Self-update to the latest release.
pub async fn run() -> anyhow::Result<()> {
    eprintln!("Current version: {}", CURRENT_VERSION);
    eprintln!("Checking for updates...");

    let client = http_client()?;

    let latest = fetch_latest_version(&client).await?;
    let latest_clean = latest.trim_start_matches('v');

    if latest_clean == CURRENT_VERSION {
        eprintln!("Already up to date.");
        return Ok(());
    }

    eprintln!(
        "New version available: {} → {}",
        CURRENT_VERSION, latest_clean
    );

    let platform = detect_platform()?;
    let artifact = format!("dw-{}", platform);
    let download_url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        REPO, latest_clean, artifact
    );
    let checksum_url = format!(
        "https://github.com/{}/releases/download/v{}/checksums.txt",
        REPO, latest_clean
    );

    // Download binary
    eprintln!("Downloading {}...", artifact);
    let binary = client
        .get(&download_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| {
            if e.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                anyhow::anyhow!(
                    "Binary '{}' not found for v{}. It may still be building — try again \
                     in a few minutes.\nRelease: https://github.com/{}/releases/tag/v{}",
                    artifact,
                    latest_clean,
                    REPO,
                    latest_clean
                )
            } else {
                anyhow::anyhow!("Download failed ({}): {}", format_http_error(&e), e)
            }
        })?
        .bytes()
        .await?;

    // Download and verify checksum
    eprintln!("Verifying checksum...");
    let checksums_text = client
        .get(&checksum_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not download checksums ({}): {}",
                format_http_error(&e),
                e
            )
        })?
        .text()
        .await?;

    // Parse checksums as "hash  filename" pairs, match exact filename
    let expected = checksums_text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let filename = parts.next()?;
            Some((hash, filename))
        })
        .find(|(_, filename)| *filename == artifact)
        .map(|(hash, _)| hash)
        .ok_or_else(|| anyhow::anyhow!("Checksum not found for {}", artifact))?;

    let mut hasher = Sha256::new();
    hasher.update(&binary);
    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        anyhow::bail!(
            "Checksum mismatch!\n  Expected: {}\n  Got:      {}\nUpdate aborted.",
            expected,
            actual
        );
    }

    // Replace the current binary
    let current_exe = std::env::current_exe()?;
    let temp_path = current_exe.with_extension("new");

    // Write to temp file next to the current binary
    let mut file = std::fs::File::create(&temp_path).map_err(|e| {
        anyhow::anyhow!(
            "Could not write to {}: {}. Try running with sudo or reinstalling.",
            temp_path.display(),
            e
        )
    })?;
    file.write_all(&binary)?;
    file.flush()?;
    drop(file);

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // Atomic rename over the current binary
    std::fs::rename(&temp_path, &current_exe).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        anyhow::anyhow!(
            "Could not replace binary at {}: {}. Try running with sudo or reinstalling.",
            current_exe.display(),
            e
        )
    })?;

    eprintln!("Updated to v{}.", latest_clean);
    Ok(())
}

async fn fetch_latest_version(client: &reqwest::Client) -> anyhow::Result<String> {
    let response: serde_json::Value = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            REPO
        ))
        .send()
        .await?
        .error_for_status()
        .map_err(|e| {
            anyhow::anyhow!(
                "Could not check for updates ({}): {}",
                format_http_error(&e),
                e
            )
        })?
        .json()
        .await?;

    response["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not parse latest version from GitHub API response"))
}

fn detect_platform() -> anyhow::Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let os_str = match os {
        "linux" => "linux",
        "macos" => "darwin",
        _ => anyhow::bail!("Unsupported OS: {}", os),
    };

    let arch_str = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => anyhow::bail!("Unsupported architecture: {}", arch),
    };

    Ok(format!("{}-{}", os_str, arch_str))
}
