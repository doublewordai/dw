# dw-cli

The [Doubleword CLI](https://github.com/doublewordai/dw) — a terminal tool for batch inference at scale.

## Install

```bash
pip install dw-cli
dw --version
```

If `dw` is not found after installing, pip's script directory may not be in your PATH. Try:

```bash
python3 -m dw_cli --version
```

Or install via the install script instead:

```bash
curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh
```

## Usage

```bash
dw login                              # Authenticate via browser
dw models list                        # List available models
dw batches run batch.jsonl --watch    # Upload, create batch, watch progress
dw realtime model "prompt"            # One-shot inference
```

## How it works

This package bundles the pre-built native `dw` binary for your platform. The `dw` command is a thin wrapper that executes the bundled binary directly — no runtime downloads or network access needed.

Supports Linux (x86_64, arm64) and macOS (x86_64, arm64).

For more information, see the [main repository](https://github.com/doublewordai/dw).
