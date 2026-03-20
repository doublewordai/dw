"""
Doubleword CLI — thin Python wrapper that downloads and runs the dw binary.

Install: pip install dw-cli
Usage:   dw login / dw batches list / dw --help
"""

import os
import platform
import stat
import subprocess
import sys
import urllib.request
import json
from pathlib import Path

# Where the binary lives after download
CACHE_DIR = Path.home() / ".dw" / "bin"
BINARY_NAME = "dw"
REPO = "doublewordai/dw"


def _get_platform_suffix() -> str:
    """Map current platform to the GitHub release artifact name suffix."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == "linux":
        os_name = "linux"
    elif system == "darwin":
        os_name = "darwin"
    elif system == "windows":
        os_name = "windows"
    else:
        raise RuntimeError(f"Unsupported OS: {system}")

    if machine in ("x86_64", "amd64"):
        arch = "amd64"
    elif machine in ("aarch64", "arm64"):
        arch = "arm64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    return f"{os_name}-{arch}"


def _get_latest_version() -> str:
    """Fetch the latest release version from GitHub."""
    url = f"https://api.github.com/repos/{REPO}/releases/latest"
    req = urllib.request.Request(url, headers={"Accept": "application/vnd.github+json"})
    with urllib.request.urlopen(req, timeout=15) as resp:
        data = json.loads(resp.read().decode())
    tag = data["tag_name"]
    # Strip leading 'v' if present
    return tag.lstrip("v")


def _get_installed_version(binary_path: Path) -> str | None:
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


def _download_binary(version: str) -> Path:
    """Download the correct binary for this platform from GitHub releases."""
    suffix = _get_platform_suffix()
    artifact = f"dw-{suffix}"
    url = f"https://github.com/{REPO}/releases/download/v{version}/{artifact}"

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    binary_path = CACHE_DIR / BINARY_NAME

    print(f"Downloading dw v{version} for {suffix}...", file=sys.stderr)

    urllib.request.urlretrieve(url, binary_path)

    # Make executable
    binary_path.chmod(binary_path.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    return binary_path


def _ensure_binary() -> Path:
    """Ensure the dw binary is downloaded and up to date."""
    binary_path = CACHE_DIR / BINARY_NAME

    installed = _get_installed_version(binary_path)

    # If binary exists and works, use it (check for updates periodically)
    if installed is not None:
        # Check for updates at most once per day
        marker = CACHE_DIR / ".last-update-check"
        import time

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
                        f"Updating dw: {installed} -> {latest}",
                        file=sys.stderr,
                    )
                    _download_binary(latest)
            except Exception:
                pass  # Network error, use existing binary

        return binary_path

    # No binary — download latest
    try:
        version = _get_latest_version()
    except Exception as e:
        print(f"Error: Could not fetch latest version: {e}", file=sys.stderr)
        print("Install manually: https://github.com/doublewordai/dw/releases", file=sys.stderr)
        sys.exit(1)

    return _download_binary(version)


def main():
    """Entry point — find/download the binary and exec it with all args."""
    binary = _ensure_binary()

    # Replace this process with the binary
    try:
        result = subprocess.run([str(binary)] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)
    except FileNotFoundError:
        print(f"Error: Binary not found at {binary}", file=sys.stderr)
        print("Try reinstalling: pip install --force-reinstall dw-cli", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
