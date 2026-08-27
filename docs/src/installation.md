# Installation

## Install Script (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh
```

This downloads the latest release binary for your platform and installs it to `~/.local/bin/`. The script detects your OS (Linux, macOS) and architecture (x86_64, arm64) automatically.

## Windows

```powershell
irm https://raw.githubusercontent.com/doublewordai/dw/main/install.ps1 | iex
```

This installs `dw.exe` to `%LOCALAPPDATA%\Programs\dw` and adds it to your PATH.

Or use a package manager:

```powershell
choco install dw
```

```powershell
scoop bucket add dw https://github.com/doublewordai/dw
scoop install dw
```

`dw` runs natively on Windows, no WSL needed. The exception is `dw project`, which runs its steps through `sh` and so needs Git Bash, WSL, or MSYS2.

## pip

```bash
pip install --user dw-cli
```

If pip warns that the script directory is not on PATH, add it:

```bash
# macOS
export PATH="$HOME/Library/Python/3.12/bin:$PATH"

# Linux
export PATH="$HOME/.local/bin:$PATH"
```

Add the appropriate line to your `~/.zshrc` or `~/.bashrc` to make it permanent.

## From Source

```bash
git clone https://github.com/doublewordai/dw.git
cd dw
cargo build --release
cp target/release/dw ~/.local/bin/
```

On Windows the built binary is `target\release\dw.exe`.

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

# PowerShell
dw completions powershell >> $PROFILE
```

## Updating

```bash
dw update
```

This downloads the latest release from GitHub, verifies the checksum, and replaces the binary in place.

On Windows the running binary cannot be overwritten, so `dw update` renames it to `dw.exe.old` first. The next update deletes that leftover file.
