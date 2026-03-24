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
            anyhow::anyhow!("Download failed ({}): {}", e.status().unwrap_or_default(), e)
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
                e.status().unwrap_or_default(),
                e
            )
        })?
        .text()
        .await?;

    let expected = checksums_text
        .lines()
        .find(|line| line.contains(&artifact))
        .and_then(|line| line.split_whitespace().next())
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

    // Write to temp file
    let mut file = std::fs::File::create(&temp_path)?;
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
        // Clean up temp file on failure
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
                e.status().unwrap_or_default(),
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
