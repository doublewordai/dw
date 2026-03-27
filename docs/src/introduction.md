# dw

The Doubleword Batch Inference CLI. Upload JSONL files, run batches, stream results, and send real-time inference requests — all from the terminal.

## Features

- **Batch processing** — upload files, create batches, watch progress, download results
- **Streaming** — `dw stream` goes from JSONL file to piped results in one command
- **Real-time inference** — one-shot streaming requests via `dw realtime`
- **Local file tools** — validate, prepare, stats, sample, merge, split, and diff JSONL files without uploading
- **Project scaffolding** — `dw project init` creates ready-to-run project templates
- **Multi-account** — manage multiple accounts and organizations, switch between them like kubectl contexts
- **Output formats** — table for interactive use, JSON for scripts, auto-detected from TTY
- **Shell completions** — bash, zsh, and fish
- **Self-update** — `dw update` downloads the latest release

## Quick Example

```bash
# Authenticate
dw login

# See available models
dw models list

# Run a batch and stream results
dw stream batch.jsonl > results.jsonl

# One-shot inference
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Explain batch inference"

# Check what the batch cost
dw batches analytics <batch-id>
```
