"""
Doubleword CLI — the dw binary, distributed via pip.

Install: pip install dw-cli
Usage:   dw login / dw batches list / dw --help
"""

import subprocess
import sys
from pathlib import Path

# The binary bundled inside this package at build time
_BUNDLED = Path(__file__).parent / "bin" / "dw"


def main():
    """Entry point — run the bundled binary with all args."""
    if not _BUNDLED.exists():
        print(
            "Error: dw binary not found in package.\n"
            "This may mean there is no pre-built binary for your platform.\n"
            "Install via: curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        result = subprocess.run([str(_BUNDLED)] + sys.argv[1:])
        sys.exit(result.returncode)
    except OSError as e:
        # Covers PermissionError, missing dynamic linker, incompatible glibc, etc.
        print(
            f"Error: Could not execute {_BUNDLED}: {e}\n"
            f"This may be a platform compatibility issue.\n"
            f"Try: chmod +x {_BUNDLED}\n"
            f"Or install via: curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh",
            file=sys.stderr,
        )
        sys.exit(1)
    except KeyboardInterrupt:
        sys.exit(130)


if __name__ == "__main__":
    main()
