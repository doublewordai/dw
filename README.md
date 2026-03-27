# dw — Doubleword Batch Inference CLI

[![GitHub](https://img.shields.io/badge/GitHub-doublewordai%2Fdw-blue)](https://github.com/doublewordai/dw)

> **[Full documentation](https://doublewordai.github.io/dw/)**

A command-line tool for the [Doubleword](https://doubleword.ai) batch inference platform. Upload JSONL files, run batches, stream results, and send real-time inference requests — all from the terminal.

Replaces curl commands and custom scripts with a single tool for managing files, running batches, and sending inference requests. Built for developers who interact with the API directly, scripts that automate batch workflows, and AI agents building data pipelines.

## Install

### curl (Linux, macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/doublewordai/dw/main/install.sh | sh
```

### pip (macOS recommended — avoids Gatekeeper issues)

```bash
pip install dw-cli
```

### Build from source

```bash
git clone https://github.com/doublewordai/dw.git
cd dw
cargo build --release
cp target/release/dw ~/.local/bin/   # or anywhere in your PATH
```

## Setup

### Login (recommended)

```bash
dw login
```

Opens your browser for SSO authentication. On completion, the CLI stores an inference key and platform key locally — giving full access to every command.

For org-scoped usage (shared billing, model access):

```bash
dw login --org acme-corp
```

### API key login (agents and CI)

For AI agents, CI pipelines, and headless environments without a browser:

```bash
dw login --api-key "sk-your-inference-key"
```

Create an inference key from [app.doubleword.ai](https://app.doubleword.ai) > Settings > API Keys. This gives access to files, batches, streaming, and real-time inference. Platform operations (models, webhooks, whoami) require the full `dw login` flow.

### Verify

```bash
dw config show         # Check server URLs and active account
dw whoami              # Show current user
dw models list         # List available models
```

### Point at a different server

```bash
# Self-hosted deployment (single domain)
dw config set-url https://your-server.example.com

# Set individually
dw config set-ai-url https://api.mycompany.com
dw config set-admin-url https://app.mycompany.com
```

## Quick Start

```bash
# Authenticate
dw login

# See what models are available
dw models list

# Send a one-shot prompt with streaming output
dw realtime Qwen3-30B "What is the capital of France?"

# Run a batch from a JSONL file and stream results to stdout
dw stream batch.jsonl > results.jsonl
```

## Authentication

### Access by auth method

| Command | `dw login` | `dw login --api-key` |
|---------|:---:|:---:|
| `dw files *` (upload, list, get, delete, content, cost-estimate) | Yes | Yes |
| `dw batches *` (create, list, get, cancel, retry, results, run, watch) | Yes | Yes |
| `dw stream` | Yes | Yes |
| `dw realtime` | Yes | Yes |
| `dw files validate / prepare` (local, no API call) | Yes | Yes |
| `dw examples *` (no auth needed) | Yes | Yes |
| `dw config *` / `dw account *` (local) | Yes | Yes |
| `dw models list / get` | Yes | — |
| `dw whoami` | Yes | — |
| `dw keys *` | Yes | — |
| `dw webhooks *` | Yes | — |
| `dw usage` | Yes | — |
| `dw requests` | Yes | — (requires RequestViewer role) |

A typical setup: you (the developer) run `dw login` to set up your account and browse models. Then create an API key with `dw keys create --name "my-bot"` and hand it to your agent or CI pipeline with `dw login --api-key`.

## Configuration

Config is stored in `~/.dw/config.toml`. Credentials in `~/.dw/credentials.toml` (permissions `0600`).

### Server URLs

By default, `dw` points to `api.doubleword.ai` (inference) and `app.doubleword.ai` (admin). Override for self-hosted deployments:

```bash
# Point both APIs to a single host
dw config set-url https://your-server.example.com

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
dw --server https://your-server.example.com models list
dw --server-ai https://custom-api.com batches list
```

### Accounts

Multiple accounts (personal + organizations) work like kubectl contexts. Each login creates a named account — auto-named from your display name (personal) or org name (organization). The name is what you see and what you type.

```bash
dw account list              # Show all stored accounts (* = active)
dw account current           # Show active account
dw account switch Doubleword # Switch to an org account
dw account switch "Hamish Main"  # Switch to personal
```

Custom names at login or rename after:

```bash
dw login --org acme-corp --as prod     # Store as "prod" instead of auto-name
dw account rename "Hamish Main" personal   # Rename for convenience
dw account remove old-account          # Delete stored credentials
```

Override per-command without switching:

```bash
dw --account Doubleword batches list
```

```bash
dw whoami          # Show current user and roles
dw logout          # Remove active account credentials
dw logout --all    # Remove all accounts
```

When you log in within an org context (`dw login --org acme-corp`), resources are billed to the org with per-member attribution.

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

## Commands

### Models

```bash
dw models list                     # List all available models
dw models list --type chat         # Filter by type
dw models get Qwen3-30B            # Get model details
```

### Pagination

List commands (`files list`, `batches list`) use cursor-based pagination. By default they return 20 items and show a hint if more are available.

```bash
dw files list                          # First 20 items
dw files list -n 50                    # Custom page size (max 100)
dw files list --after <last-id>        # Next page (use ID from hint)
dw files list --all                    # Fetch everything (auto-paginates)
```

In table/plain mode, a hint prints to stderr when more items exist:
```
More files available. Next page: dw files list --after <id>
```

In JSON mode the hint is suppressed — pipe-friendly. Get the cursor from the last object:
```bash
last_id=$(dw files list --output json | tail -1 | jq -r '.id')
dw files list --after "$last_id" --output json
```

The same flags (`-n`, `--after`, `--all`) work identically on `dw batches list`.

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
dw files prepare batch.jsonl --model Qwen3-235B --output-file prepared.jsonl

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

#### JSONL Inspection & Manipulation

```bash
# Inspect a batch file before submitting
dw files stats batch.jsonl                        # Line count, models, estimated tokens

# Sample a subset for testing
dw files sample batch.jsonl -n 50 -o test.jsonl   # Random 50 lines

# Merge multiple files
dw files merge batch-a.jsonl batch-b.jsonl -o combined.jsonl

# Split a large file into chunks
dw files split large.jsonl --chunk-size 1000       # → large-001.jsonl, large-002.jsonl, ...

# Compare results from two models
dw files diff results-30b.jsonl results-235b.jsonl
```

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

Add `--usage` to print token counts to stderr after completion. Off by default to keep output clean for piping.

### Webhooks

Get notified when batches complete or fail.

```bash
dw webhooks create --url https://example.com/hook --events batch.completed,batch.failed
dw webhooks list
dw webhooks delete <webhook-id>
dw webhooks rotate-secret <webhook-id>
```

### API Keys

Create and manage API keys for scripts, bots, and integrations.

```bash
dw keys create --name "my-bot"          # Create a key (secret shown once)
dw keys list                            # List your keys (secrets masked)
dw keys delete <key-id>                 # Revoke a key
```

Created keys are `realtime` purpose — use them with the OpenAI SDK or `dw login --api-key`.

### Examples

Browse and clone production-ready examples from the Doubleword use-cases repository.

```bash
dw examples list                                 # Browse available examples
dw examples clone synthetic-data-generation      # Clone to ./synthetic-data-generation/
dw examples clone model-evals --dir ./my-evals   # Clone to custom directory
```

Each example includes working code, sample data, and a README with results and costs.

### Project Steps

When inside a cloned example (or any project with a `dw.toml`), run project-specific steps:

```bash
dw project setup                                 # Install dependencies (e.g. uv sync)
dw project info                                  # Show available steps
dw project run prepare --model 30b -n 100        # Run a named step with args
dw project run analyze --results results.jsonl   # Run another step
```

A typical example workflow:

```bash
dw examples clone model-evals && cd model-evals
dw project setup
dw project run prepare -n 100                    # Python generates batch JSONL
dw files stats output/batch.jsonl                # Inspect before submitting
dw files prepare output/batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8
dw stream output/batch.jsonl > results.jsonl     # Submit, watch, stream results
dw project run analyze --results results.jsonl   # Score/analyze results
dw usage                                         # See what it cost
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

## Client Settings

Optional `[client]` section in `~/.dw/config.toml` for tuning HTTP behavior:

```toml
[client]
timeout_secs = 300          # Request timeout (default: 300s / 5 min)
connect_timeout_secs = 10   # TCP connect timeout (default: 10s)
max_retries = 1             # Retries on transient errors (default: 1, max: 10, 0 = disabled)
poll_interval_secs = 2      # Polling interval for watch/stream (default: 2s, min: 1s)
```

All fields are optional with sensible defaults. Omit the entire `[client]` section or individual fields — defaults apply per-field. Values are clamped: `max_retries` to 0–10, `poll_interval_secs` to minimum 1s, timeouts to minimum 1s.

## Development

```bash
just build     # Build both crates
just test      # Run tests
just lint      # Clippy + fmt check
just ci        # lint + test
just run -- models list   # Run with args
```

## Documentation

Full documentation is available at **[doublewordai.github.io/dw](https://doublewordai.github.io/dw/)**.

- [Installation](https://doublewordai.github.io/dw/installation.html)
- [Quickstart](https://doublewordai.github.io/dw/quickstart.html)
- [Batch Processing](https://doublewordai.github.io/dw/batches.html)
- [JSONL Format](https://doublewordai.github.io/dw/jsonl-format.html)
- [Local File Tools](https://doublewordai.github.io/dw/file-tools.html)
- [Project System](https://doublewordai.github.io/dw/projects.html)
- [Examples](https://doublewordai.github.io/dw/examples.html)
- [Command Reference](https://doublewordai.github.io/dw/commands.html)
