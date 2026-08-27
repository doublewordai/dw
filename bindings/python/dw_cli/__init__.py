"""
Doubleword CLI — the dw binary, distributed via pip.

Install: pip install dw-cli
Usage:   dw login / dw batches list / dw --help
"""

import subprocess
import sys
from pathlib import Path

_IS_WINDOWS = sys.platform == "win32"

# The binary bundled into this package at build time. The name here has to
# match bundled_binary_name() in build_wheel.py.
_BUNDLED = Path(__file__).parent / "bin" / ("dw.exe" if _IS_WINDOWS else "dw")

_INSTALL_SH = (
    "curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh"
)

# Where to get dw by hand if the bundled binary is missing or will not run.
_INSTALL_HINT = (
    "Download dw-windows-amd64.exe from https://github.com/doublewordai/dw/releases"
    if _IS_WINDOWS
    else f"Install via: {_INSTALL_SH}"
)


def main():
    """Entry point — run the bundled binary with all args."""
    if not _BUNDLED.exists():
        print(
            "Error: dw binary not found in package.\n"
            "This may mean there is no pre-built binary for your platform.\n"
            f"{_INSTALL_HINT}",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        result = subprocess.run([str(_BUNDLED)] + sys.argv[1:])
        sys.exit(result.returncode)
    except OSError as e:
        # Covers PermissionError, missing dynamic linker, incompatible glibc, etc.
        chmod_hint = "" if _IS_WINDOWS else f"Try: chmod +x {_BUNDLED}\n"
        print(
            f"Error: Could not execute {_BUNDLED}: {e}\n"
            f"This may be a platform compatibility issue.\n"
            f"{chmod_hint}"
            f"{_INSTALL_HINT}",
            file=sys.stderr,
        )
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(130)


if __name__ == "__main__":
    main()
