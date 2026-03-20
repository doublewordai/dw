# dw-cli

Python wrapper for the [Doubleword CLI](https://github.com/doublewordai/dw) — a terminal tool for batch inference at scale.

## Install

```bash
pip install dw-cli
```

This installs a `dw` command that automatically downloads the correct native binary for your platform on first run.

## Usage

```bash
dw login                              # Authenticate via browser
dw models list                        # List available models
dw files upload batch.jsonl           # Upload a JSONL file
dw batches run batch.jsonl --watch    # Upload, create batch, and watch progress
dw stream batch.jsonl > results.jsonl # Stream results as they complete
dw realtime model "prompt"            # One-shot inference
```

## How it works

This package is a thin wrapper. On first run it downloads the pre-built native binary from [GitHub Releases](https://github.com/doublewordai/dw/releases) to `~/.dw/bin/` and executes it. Updates are checked daily.

For direct binary installation without Python, see the [main repository](https://github.com/doublewordai/dw).
