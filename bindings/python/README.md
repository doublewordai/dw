# dw-cli

The [Doubleword CLI](https://github.com/doublewordai/dw) — a terminal tool for batch inference at scale.

## Install

```bash
pip install dw-cli
dw --version
```

If `dw` is not found after installing, run this once to bootstrap:

```bash
python3 -m dw_cli --version
```

This copies the binary to `~/.local/bin/`. Add it to your PATH if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

```bash
dw login                              # Authenticate via browser
dw models list                        # List available models
dw batches run batch.jsonl --watch    # Upload, create batch, watch progress
dw realtime model "prompt"            # One-shot inference
```

## How it works

This package bundles the pre-built native `dw` binary for your platform. On first run, it installs the binary to `~/.local/bin/` so it's available system-wide. No runtime downloads or network access needed.

For direct installation without Python, see the [main repository](https://github.com/doublewordai/dw).
