# dw — Doubleword Batch Inference CLI

A command-line tool for the [Doubleword](https://doubleword.ai) batch inference platform. Upload JSONL files, run batches, stream results, and send real-time inference requests — all from the terminal.

Designed for developers scripting batch workflows and AI agents building data pipelines.

## Install

```bash
# Download and install the binary
curl -fsSL https://doubleword.ai/install.sh | sh

# Or build from source
cargo install --path crates/dw-cli
```

## Quick Start

```bash
# Authenticate with an API key (get one from app.doubleword.ai)
dw login --api-key "sk-your-key"

# See what models are available
dw models list

# Run a batch from a JSONL file and stream results to stdout
dw stream batch.jsonl

# Send a one-shot prompt
dw realtime Qwen3-30B "What is the capital of France?"
```

## Authentication

**API key** (recommended for scripting and agents):

```bash
dw login --api-key "sk-your-key"
```

**Browser login** (full access — requires the control-layer callback endpoint):

```bash
dw login
```

Browser login creates both a realtime key (for inference, files, batches) and a platform key (for webhooks, account info). API key login stores just the key you provide.

```bash
dw whoami          # Show current user and credit balance
dw logout          # Remove stored credentials
dw logout --all    # Remove all accounts
```

## Configuration

Config is stored in `~/.dw/config.toml`. Credentials in `~/.dw/credentials.toml` (permissions `0600`).

### Server URLs

By default, `dw` points to `api.doubleword.ai` (inference) and `app.doubleword.ai` (admin). Override for self-hosted or staging deployments:

```bash
# Point both APIs to a single host (e.g. staging)
dw config set-url https://staging.doubleword.ai

# Set individually
dw config set-ai-url https://api.mycompany.com
dw config set-admin-url https://app.mycompany.com

# Reset to defaults
dw config reset-urls

# Show current config
dw config show
```

You can also override per-command without persisting:

```bash
dw --server https://staging.doubleword.ai models list
dw --server-ai https://custom-api.com batches list
```

### Accounts

Multiple accounts (personal + organizations) work like kubectl contexts:

```bash
dw account list           # Show all stored accounts
dw account current        # Show active account
dw account switch acme    # Switch to org account
dw --account personal batches list   # Override per-command
```

When you log in within an org context, resources are billed to the org with per-member attribution.

## Commands

### Models

```bash
dw models list                     # List all available models
dw models list --type chat         # Filter by type
dw models get Qwen3-30B            # Get model details
```

### Files

Upload JSONL files for batch processing, with optional transforms applied before upload.

```bash
dw files upload batch.jsonl                    # Upload as-is
dw files upload batch.jsonl --model Qwen3-30B  # Override model on every line
dw files upload batch.jsonl --temperature 0.7  # Set temperature

dw files list                      # List uploaded files
dw files get <file-id>             # File metadata
dw files content <file-id>         # Download file content
dw files cost-estimate <file-id>   # Estimated cost before running
dw files delete <file-id>          # Delete (prompts for confirmation)
```

### JSONL Preparation

Transform JSONL files locally without uploading. Useful for preparing files before submission, or for agents building batch pipelines.

```bash
# Override the model
dw files prepare batch.jsonl --model Qwen3-235B --output prepared.jsonl

# Set generation parameters
dw files prepare batch.jsonl --temperature 0.5 --max-tokens 200 --top-p 0.9

# Set arbitrary fields with dot-notation
dw files prepare batch.jsonl --set body.stream=false --set body.seed=42

# Encode local images to base64 data URIs
dw files prepare batch.jsonl --encode-images

# Add or remove lines
dw files prepare batch.jsonl --add-line '{"custom_id":"extra","method":"POST","url":"/v1/chat/completions","body":{"model":"x","messages":[{"role":"user","content":"Hi"}]}}'
dw files prepare batch.jsonl --remove-lines "test-"

# Validate without modifying
dw files validate batch.jsonl
```

Transforms can also be applied inline during upload: `dw files upload batch.jsonl --model Qwen3-30B`.

### Batches

```bash
# Create a batch from an uploaded file
dw batches create --file <file-id> --completion-window 24h

# Upload and create in one step
dw batches run batch.jsonl
dw batches run batch.jsonl --model Qwen3-30B --watch  # Override model + watch
dw batches run ./batch-dir/                            # All .jsonl files in a directory

# Monitor
dw batches list --active-first
dw batches get <batch-id>
dw batches watch <batch-id>          # Live progress until completion

# Results
dw batches results <batch-id>                        # Print to stdout
dw batches results <batch-id> -o results.jsonl       # Save to file

# Manage
dw batches cancel <batch-id>
dw batches retry <batch-id>
```

### Stream

The `stream` command combines upload, batch creation, progress watching, and result output into a single step. Progress goes to stderr, results to stdout — so you can pipe directly.

```bash
dw stream batch.jsonl                              # Results to stdout
dw stream batch.jsonl > results.jsonl              # Results to file
dw stream batch.jsonl --model Qwen3-235B           # Override model
dw stream batch.jsonl --completion-window 1h       # Faster SLA
dw stream ./batches/                               # All files in directory
```

### Real-Time Inference

Send one-shot prompts with streaming output. Tokens print as they arrive.

```bash
dw realtime Qwen3-30B "Explain quantum computing in one sentence"

# With system message and parameters
dw realtime Qwen3-30B "Write a haiku about rust" \
  --system "You are a poet" --temperature 0.9 --max-tokens 50

# Pipe input from stdin
echo "Translate to French: Hello world" | dw realtime Qwen3-30B
cat article.txt | dw realtime Qwen3-30B "Summarize this"

# Save output
dw realtime Qwen3-30B "List 10 animals" -o animals.txt

# Non-streaming mode
dw realtime Qwen3-30B "Hello" --no-stream
```

Token usage is printed to stderr after completion, so stdout stays clean for piping.

### Webhooks

Get notified when batches complete or fail.

```bash
dw webhooks create --url https://example.com/hook --events batch.completed,batch.failed
dw webhooks list
dw webhooks delete <webhook-id>
dw webhooks rotate-secret <webhook-id>
```

### Examples

Browse and clone production-ready examples from the Doubleword use-cases repository.

```bash
dw examples list                                 # Browse available examples
dw examples clone synthetic-data-generation      # Clone to ./synthetic-data-generation/
dw examples clone model-evals --dir ./my-evals   # Clone to custom directory
```

Each example includes working code, sample data, and a README with results and costs.

## Output Formats

Every list and detail command supports three output formats:

```bash
dw models list --output table    # Human-readable table (default for TTY)
dw models list --output json     # One JSON object per line (default for pipes)
dw models list --output plain    # Minimal, tab-separated
```

Auto-detection: if stdout is a terminal, defaults to `table`. If piped, defaults to `json`.

```bash
# These are equivalent:
dw batches list --output json
dw batches list | cat
```

## Global Flags

| Flag | Description |
|------|-------------|
| `--output table\|json\|plain` | Output format |
| `--account <name>` | Use a specific account for this command |
| `--server <url>` | Override both API URLs for this command |
| `--server-ai <url>` | Override inference API URL |
| `--server-admin <url>` | Override admin API URL |
| `--help` | Help for any command |
| `--version` | Print version |

## Shell Completions

```bash
# Bash
eval "$(dw completions bash)"

# Zsh
eval "$(dw completions zsh)"

# Fish
dw completions fish | source
```

Add to your shell config for persistence.

## Architecture

`dw` is built as two Rust crates:

- **`dw-client`** — API client library. Typed endpoints for files, batches, models, webhooks. Reusable outside the CLI.
- **`dw-cli`** — Binary with command parsing, config management, JSONL tools, and output formatting.

The client maintains two API surfaces internally (inference at `/ai/v1/*` and admin at `/admin/api/v1/*`), routing each request to the correct server with the appropriate key. This split is invisible to the user — it just works.

## Development

```bash
just build     # Build both crates
just test      # Run tests
just lint      # Clippy + fmt check
just ci        # lint + test
just run -- models list   # Run with args
```
