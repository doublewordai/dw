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
    let artifact = artifact_name(&platform);
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
    let temp_path = sibling_path(&current_exe, ".new");

    // Write to temp file next to the current binary
    let mut file = std::fs::File::create(&temp_path).map_err(|e| {
        anyhow::anyhow!(
            "Could not write to {}: {}. {}",
            temp_path.display(),
            e,
            replace_failure_hint()
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

    // Windows will not let us overwrite the binary while it is running, but it
    // will let us rename it. So move the old one aside and put the new one in
    // its place.
    #[cfg(windows)]
    {
        let old_path = sibling_path(&current_exe, ".old");

        // Left behind by an earlier update. It can be deleted now that the
        // process using it has exited.
        let _ = std::fs::remove_file(&old_path);

        std::fs::rename(&current_exe, &old_path).map_err(|e| {
            let _ = std::fs::remove_file(&temp_path);
            anyhow::anyhow!(
                "Could not move the running binary at {} aside: {}. {}",
                current_exe.display(),
                e,
                replace_failure_hint()
            )
        })?;

        if let Err(e) = std::fs::rename(&temp_path, &current_exe) {
            // Put the old binary back so the install still works.
            let _ = std::fs::rename(&old_path, &current_exe);
            let _ = std::fs::remove_file(&temp_path);
            anyhow::bail!(
                "Could not replace binary at {}: {}. {}",
                current_exe.display(),
                e,
                replace_failure_hint()
            );
        }

        // This fails while dw is still running, which is expected. The next
        // update deletes it.
        let _ = std::fs::remove_file(&old_path);
    }

    // Atomic rename over the current binary
    #[cfg(not(windows))]
    std::fs::rename(&temp_path, &current_exe).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        anyhow::anyhow!(
            "Could not replace binary at {}: {}. {}",
            current_exe.display(),
            e,
            replace_failure_hint()
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
    platform_string(std::env::consts::OS, std::env::consts::ARCH)
}

/// Turns an OS and architecture into the platform name used in release
/// filenames. Takes both as arguments so every target can be tested, not just
/// the one this binary was built for.
fn platform_string(os: &str, arch: &str) -> anyhow::Result<String> {
    let os_str = match os {
        "linux" => "linux",
        "macos" => "darwin",
        "windows" => "windows",
        _ => anyhow::bail!("Unsupported OS: {}", os),
    };

    let arch_str = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => anyhow::bail!("Unsupported architecture: {}", arch),
    };

    Ok(format!("{}-{}", os_str, arch_str))
}

/// The filename to download from the release. This has to match exactly what
/// release-please.yml uploads, because it is used both in the download URL and
/// to look up the checksum.
fn artifact_name(platform: &str) -> String {
    if platform.starts_with("windows-") {
        format!("dw-{}.exe", platform)
    } else {
        format!("dw-{}", platform)
    }
}

/// Adds a suffix to a filename, keeping any extension it already has, so
/// `dw.exe` becomes `dw.exe.new`. `Path::with_extension` would give `dw.new`.
fn sibling_path(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

/// What to suggest when the binary cannot be written or replaced. The usual
/// cause differs by platform: a file lock on Windows, permissions elsewhere.
fn replace_failure_hint() -> &'static str {
    if cfg!(windows) {
        "Close any other running dw processes and try again, or reinstall."
    } else {
        "Try running with sudo or reinstalling."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Keep in sync with the `build-release` matrix in release-please.yml.
    const SUPPORTED: &[(&str, &str, &str)] = &[
        ("linux", "x86_64", "linux-amd64"),
        ("linux", "aarch64", "linux-arm64"),
        ("macos", "x86_64", "darwin-amd64"),
        ("macos", "aarch64", "darwin-arm64"),
        ("windows", "x86_64", "windows-amd64"),
    ];

    #[test]
    fn platform_string_maps_every_released_target() {
        for (os, arch, expected) in SUPPORTED {
            assert_eq!(platform_string(os, arch).unwrap(), *expected);
        }
    }

    #[test]
    fn platform_string_rejects_unsupported_os_and_arch() {
        assert!(platform_string("freebsd", "x86_64").is_err());
        assert!(platform_string("linux", "riscv64").is_err());
    }

    /// Fails if CI starts building on a platform we have no mapping for.
    #[test]
    fn detect_platform_succeeds_on_this_host() {
        assert!(detect_platform().is_ok());
    }

    #[test]
    fn artifact_name_matches_released_filenames() {
        assert_eq!(artifact_name("linux-amd64"), "dw-linux-amd64");
        assert_eq!(artifact_name("linux-arm64"), "dw-linux-arm64");
        assert_eq!(artifact_name("darwin-amd64"), "dw-darwin-amd64");
        assert_eq!(artifact_name("darwin-arm64"), "dw-darwin-arm64");
        assert_eq!(artifact_name("windows-amd64"), "dw-windows-amd64.exe");
    }

    #[test]
    fn artifact_names_keep_the_dw_prefix() {
        // The release workflow globs on `dw-*`, so a name without that prefix
        // never reaches the release and `dw update` breaks without warning.
        for (os, arch, _) in SUPPORTED {
            let name = artifact_name(&platform_string(os, arch).unwrap());
            assert!(name.starts_with("dw-"), "{name} lost the dw- prefix");
        }
    }

    /// Looks up a checksum the same way `run` does, against a copy of the
    /// checksums.txt the release workflow produces, to confirm the artifact
    /// names on both sides match.
    #[test]
    fn artifact_name_resolves_against_generated_checksums() {
        let checksums = "\
aaaa  dw-linux-amd64
bbbb  dw-linux-arm64
cccc  dw-darwin-amd64
dddd  dw-darwin-arm64
eeee  dw-windows-amd64.exe
ffff  checksums.txt
";
        let lookup = |artifact: &str| -> Option<&str> {
            checksums
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    Some((parts.next()?, parts.next()?))
                })
                .find(|(_, filename)| *filename == artifact)
                .map(|(hash, _)| hash)
        };

        assert_eq!(lookup(&artifact_name("windows-amd64")), Some("eeee"));
        assert_eq!(lookup(&artifact_name("linux-amd64")), Some("aaaa"));
        assert_eq!(lookup("dw-windows-amd64"), None, "extensionless must miss");
    }

    #[test]
    fn sibling_path_appends_instead_of_replacing_extension() {
        // with_extension would turn dw.exe into dw.new, leaving a temp file
        // Windows will not run.
        assert_eq!(
            sibling_path(Path::new("/opt/bin/dw.exe"), ".new"),
            Path::new("/opt/bin/dw.exe.new")
        );
        assert_eq!(
            sibling_path(Path::new("/opt/bin/dw.exe"), ".old"),
            Path::new("/opt/bin/dw.exe.old")
        );
        assert_eq!(
            sibling_path(Path::new("/usr/local/bin/dw"), ".new"),
            Path::new("/usr/local/bin/dw.new")
        );
    }
}
