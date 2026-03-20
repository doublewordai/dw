"""
Doubleword CLI — thin Python wrapper that downloads and runs the dw binary.

Install: pip install dw-cli
Usage:   dw login / dw batches list / dw --help
"""

import json
import platform
import stat
import subprocess
import sys
import time
import urllib.request
import urllib.error
from pathlib import Path
from typing import Optional

# Where the binary lives after download
CACHE_DIR = Path.home() / ".dw" / "bin"
BINARY_NAME = "dw"
REPO = "doublewordai/dw"


def _get_platform_suffix():
    # type: () -> str
    """Map current platform to the GitHub release artifact name suffix."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "linux":
        os_name = "linux"
    elif system == "darwin":
        os_name = "darwin"
    else:
        raise RuntimeError(
            "Unsupported OS: {}. dw-cli supports Linux and macOS. "
            "See https://github.com/doublewordai/dw/releases".format(system)
        )

    if machine in ("x86_64", "amd64"):
        arch = "amd64"
    elif machine in ("aarch64", "arm64"):
        arch = "arm64"
    else:
        raise RuntimeError(
            "Unsupported architecture: {}. "
            "See https://github.com/doublewordai/dw/releases".format(machine)
        )

    return "{}-{}".format(os_name, arch)


def _get_latest_version():
    # type: () -> str
    """Fetch the latest release version from GitHub."""
    url = "https://api.github.com/repos/{}/releases/latest".format(REPO)
    req = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode())
    tag = data["tag_name"]
    # Strip leading 'v' if present
    return tag.lstrip("v")


def _get_installed_version(binary_path):
    # type: (Path) -> Optional[str]
    """Get the version of the installed binary, if any."""
    if not binary_path.exists():
        return None
    try:
        result = subprocess.run(
            [str(binary_path), "--version"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        # Output is like "dw 0.1.0"
        parts = result.stdout.strip().split()
        return parts[-1] if parts else None
    except Exception:
        return None


def _verify_checksum(binary_path, version, suffix):
    # type: (Path, str, str) -> None
    """Download checksums.txt and verify the binary's SHA256."""
    import hashlib

    checksum_url = "https://github.com/{}/releases/download/v{}/checksums.txt".format(REPO, version)
    try:
        req = urllib.request.Request(checksum_url)
        with urllib.request.urlopen(req, timeout=15) as resp:
            checksums = resp.read().decode()
    except (urllib.error.URLError, urllib.error.HTTPError):
        print("Warning: Could not download checksums, skipping verification", file=sys.stderr)
        return

    artifact_name = "dw-{}".format(suffix)
    expected = None
    for line in checksums.strip().splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[1] == artifact_name:
            expected = parts[0]
            break

    if expected is None:
        print("Warning: No checksum found for {}, skipping verification".format(artifact_name), file=sys.stderr)
        return

    sha256 = hashlib.sha256()
    with open(binary_path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha256.update(chunk)
    actual = sha256.hexdigest()

    if actual != expected:
        binary_path.unlink(missing_ok=True)
        print(
            "Error: Checksum verification failed for dw v{}.\n"
            "  Expected: {}\n"
            "  Got:      {}\n"
            "Install manually: https://github.com/doublewordai/dw/releases".format(version, expected, actual),
            file=sys.stderr,
        )
        sys.exit(1)


def _download_binary(version):
    # type: (str) -> Path
    """Download the correct binary for this platform from GitHub releases."""
    suffix = _get_platform_suffix()
    artifact = "dw-{}".format(suffix)
    url = "https://github.com/{}/releases/download/v{}/{}".format(REPO, version, artifact)

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    binary_path = CACHE_DIR / BINARY_NAME

    print("Downloading dw v{} for {}...".format(version, suffix), file=sys.stderr)

    try:
        urllib.request.urlretrieve(url, binary_path)
    except urllib.error.HTTPError as e:
        print(
            "Error: Failed to download dw v{} for {} (HTTP {}).\n"
            "Install manually: https://github.com/doublewordai/dw/releases".format(version, suffix, e.code),
            file=sys.stderr,
        )
        sys.exit(1)
    except urllib.error.URLError as e:
        print(
            "Error: Network error downloading dw: {}\n"
            "Install manually: https://github.com/doublewordai/dw/releases".format(e.reason),
            file=sys.stderr,
        )
        sys.exit(1)

    # Verify checksum
    _verify_checksum(binary_path, version, suffix)

    # Make executable
    binary_path.chmod(binary_path.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    return binary_path


def _ensure_binary():
    # type: () -> Path
    """Ensure the dw binary is downloaded and up to date."""
    binary_path = CACHE_DIR / BINARY_NAME

    installed = _get_installed_version(binary_path)

    # If binary exists and works, use it (check for updates periodically)
    if installed is not None:
        # Check for updates at most once per day
        marker = CACHE_DIR / ".last-update-check"

        should_check = True
        if marker.exists():
            age = time.time() - marker.stat().st_mtime
            should_check = age > 86400  # 24 hours

        if should_check:
            try:
                latest = _get_latest_version()
                marker.touch()
                if latest != installed:
                    print(
                        "Updating dw: {} -> {}".format(installed, latest),
                        file=sys.stderr,
                    )
                    _download_binary(latest)
            except (urllib.error.URLError, urllib.error.HTTPError) as e:
                print(
                    "Warning: Update check failed ({}), using existing binary".format(e),
                    file=sys.stderr,
                )
            except Exception as e:
                print(
                    "Warning: Update check failed ({}), using existing binary".format(e),
                    file=sys.stderr,
                )

        return binary_path

    # No binary — download latest
    try:
        version = _get_latest_version()
    except Exception as e:
        print("Error: Could not fetch latest version: {}".format(e), file=sys.stderr)
        print("Install manually: https://github.com/doublewordai/dw/releases", file=sys.stderr)
        sys.exit(1)

    return _download_binary(version)


def main():
    """Entry point — find/download the binary and run it with all args."""
    binary = _ensure_binary()

    try:
        result = subprocess.run([str(binary)] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)
    except FileNotFoundError:
        print("Error: Binary not found at {}".format(binary), file=sys.stderr)
        print("Try reinstalling: pip install --force-reinstall dw-cli", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
