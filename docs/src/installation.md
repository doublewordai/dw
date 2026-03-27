# Installation

## Install Script (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh
```

This downloads the latest release binary for your platform and installs it to `~/.local/bin/`. The script detects your OS (Linux, macOS) and architecture (x86_64, arm64) automatically.

## pip

```bash
pip install dw-cli
```

The `dw` binary is bundled as a Python package for convenience. No Python runtime is needed — the package just contains the pre-compiled binary.

## From Source

```bash
git clone https://github.com/doublewordai/dw.git
cd dw
cargo build --release
cp target/release/dw ~/.local/bin/
```

Requires Rust 2024 edition (1.85+).

## Verify

```bash
dw --version
```

## Shell Completions

Generate shell completions for your shell:

```bash
# Bash
dw completions bash > ~/.local/share/bash-completion/completions/dw

# Zsh
dw completions zsh > ~/.zfunc/_dw

# Fish
dw completions fish > ~/.config/fish/completions/dw.fish
```

## Updating

```bash
dw update
```

This downloads the latest release from GitHub, verifies the checksum, and replaces the binary in place.
