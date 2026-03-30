#!/usr/bin/env python3
"""Build a platform-specific wheel with the dw binary bundled inside.

Usage:
    python build_wheel.py <binary-path> <platform-tag>

Example:
    python build_wheel.py ../../target/release/dw macosx_11_0_arm64
    python build_wheel.py /path/to/dw-linux-amd64 manylinux_2_17_x86_64

The binary is copied into dw_cli/bin/dw and a wheel is built with the
given platform tag. The resulting wheel is in dist/.
"""

import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path


PLATFORM_TAGS = {
    "linux-amd64": "manylinux_2_17_x86_64.manylinux2014_x86_64",
    "linux-arm64": "manylinux_2_17_aarch64.manylinux2014_aarch64",
    "darwin-amd64": "macosx_11_0_x86_64",
    "darwin-arm64": "macosx_11_0_arm64",
}


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <binary-path> <platform-key>")
        print(f"Platform keys: {', '.join(PLATFORM_TAGS.keys())}")
        sys.exit(1)

    binary_path = Path(sys.argv[1])
    platform_key = sys.argv[2]

    if platform_key not in PLATFORM_TAGS:
        print(f"Unknown platform: {platform_key}")
        print(f"Available: {', '.join(PLATFORM_TAGS.keys())}")
        sys.exit(1)

    platform_tag = PLATFORM_TAGS[platform_key]

    if not binary_path.exists():
        print(f"Binary not found: {binary_path}")
        sys.exit(1)

    # Copy binary into package
    pkg_dir = Path(__file__).parent
    bin_dir = pkg_dir / "dw_cli" / "bin"
    bin_dir.mkdir(exist_ok=True)

    target = bin_dir / "dw"
    shutil.copy2(binary_path, target)
    target.chmod(target.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    print(f"Copied {binary_path} -> {target} ({target.stat().st_size:,} bytes)")

    # Clean previous builds
    for d in ["dist", "build", "dw_cli.egg-info"]:
        p = pkg_dir / d
        if p.exists():
            shutil.rmtree(p)

    # Build wheel with platform tag
    # We use bdist_wheel directly with --plat-name to set the platform tag
    subprocess.check_call(
        [
            sys.executable, "-m", "pip", "install", "setuptools", "wheel",
        ],
        cwd=pkg_dir,
    )
    subprocess.check_call(
        [
            sys.executable, "setup.py",
            "bdist_wheel",
            "--plat-name", platform_tag,
        ],
        cwd=pkg_dir,
    )

    # List built wheels
    dist_dir = pkg_dir / "dist"
    for whl in dist_dir.glob("*.whl"):
        print(f"Built: {whl.name} ({whl.stat().st_size / 1024 / 1024:.1f} MB)")

    # Clean up the binary from the source tree
    target.unlink()
    bin_dir.rmdir()


if __name__ == "__main__":
    main()
