"""
Doubleword CLI — the dw binary, distributed via pip.

Install: pip install dw-cli
Usage:   dw login / dw batches list / dw --help
"""

import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

# Where pip bundles the binary inside the package
_BUNDLED = Path(__file__).parent / "bin" / "dw"

# Where we install it so it's on a standard PATH
_INSTALL_DIR = Path.home() / ".local" / "bin"
_INSTALLED = _INSTALL_DIR / "dw"


def _ensure_installed() -> Path:
    """Ensure the dw binary is installed to ~/.local/bin/.

    On first run after pip install, copies the bundled binary to ~/.local/bin/dw.
    On subsequent runs, uses the installed binary directly. Updates the installed
    binary if the bundled version is newer (i.e., after pip install --upgrade).
    """
    if not _BUNDLED.exists():
        print(
            "Error: dw binary not found in package.\n"
            "This may mean there is no pre-built binary for your platform.\n"
            "Install via: curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh",
            file=sys.stderr,
        )
        sys.exit(1)

    # Check if we need to install or update
    needs_install = False
    if not _INSTALLED.exists():
        needs_install = True
    elif _BUNDLED.stat().st_size != _INSTALLED.stat().st_size:
        # Size differs — bundled version was updated (pip upgrade)
        needs_install = True

    if needs_install:
        _INSTALL_DIR.mkdir(parents=True, exist_ok=True)
        shutil.copy2(_BUNDLED, _INSTALLED)
        _INSTALLED.chmod(
            _INSTALLED.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH
        )

        # Check if ~/.local/bin is in PATH
        path_dirs = os.environ.get("PATH", "").split(os.pathsep)
        if str(_INSTALL_DIR) not in path_dirs:
            shell = os.environ.get("SHELL", "")
            if "zsh" in shell:
                rc = "~/.zshrc"
            elif "bash" in shell:
                rc = "~/.bashrc"
            else:
                rc = "your shell profile"
            print(
                f"Installed dw to {_INSTALLED}\n"
                f"Add to PATH: export PATH=\"{_INSTALL_DIR}:$PATH\"\n"
                f"Add this line to {rc}, then restart your terminal.\n",
                file=sys.stderr,
            )
        else:
            print(f"Installed dw to {_INSTALLED}", file=sys.stderr)

    return _INSTALLED


def main():
    """Entry point — ensure binary is installed and run it."""
    binary = _ensure_installed()

    try:
        result = subprocess.run([str(binary)] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)


if __name__ == "__main__":
    main()
