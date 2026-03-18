# DW CLI — Testing Playbook

Test against staging or production using an API key (no browser login needed).

## Setup

```bash
# Build the binary
cd dw/
cargo build

# Alias for convenience
alias dw="./target/debug/dw"

# Or add to PATH
export PATH="$PWD/target/debug:$PATH"
```

## 1. First-Run Experience

```bash
# Should show help with command groups
dw --help

# Should show version
dw --version

# Should error with helpful message (no credentials yet)
dw whoami
dw models list
```

## 2. Login with API Key

Get a realtime API key from app.doubleword.ai (Settings > API Keys).

```bash
# Login with key — should validate and store
dw login --api-key "sk-your-key-here"

# Verify stored credentials
cat ~/.dw/credentials.toml

# Check permissions (should be 600)
ls -la ~/.dw/credentials.toml

# Check active account
dw account current
dw account list
```

## 3. Models (requires platform key — browser login)

These commands require a platform key. With API key auth, they will error with "No platform API key configured."

```bash
# List all available models
dw models list

# JSON output (for piping)
dw models list --output json

# Plain output
dw models list --output plain

# Get specific model (use an alias from the list)
dw models get <model-alias>
```

## 4. Local JSONL Tools (No API Calls)

Create a test JSONL file first:

```bash
cat > /tmp/test.jsonl << 'EOF'
{"custom_id": "req-001", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8", "messages": [{"role": "user", "content": "What is 2+2?"}], "max_tokens": 100}}
{"custom_id": "req-002", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8", "messages": [{"role": "user", "content": "What is the capital of France?"}], "max_tokens": 100}}
{"custom_id": "req-003", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8", "messages": [{"role": "user", "content": "Explain quantum computing in one sentence."}], "max_tokens": 200}}
EOF
```

### Validate

```bash
# Should pass validation
dw files validate /tmp/test.jsonl

# Create an invalid file and validate
echo '{"bad": "json"' > /tmp/bad.jsonl
dw files validate /tmp/bad.jsonl

# Missing fields
echo '{"custom_id": "x"}' > /tmp/missing.jsonl
dw files validate /tmp/missing.jsonl
```

### Prepare (Transform)

```bash
# Override model on all lines
dw files prepare /tmp/test.jsonl --model "new-model-alias" --output /tmp/transformed.jsonl
cat /tmp/transformed.jsonl | head -1 | python3 -m json.tool

# Override temperature and max_tokens
dw files prepare /tmp/test.jsonl --temperature 0.5 --max-tokens 50 --output /tmp/params.jsonl
cat /tmp/params.jsonl | head -1 | python3 -m json.tool

# Set arbitrary field
dw files prepare /tmp/test.jsonl --set body.top_p=0.9 --set body.stream=false --output /tmp/custom.jsonl
cat /tmp/custom.jsonl | head -1 | python3 -m json.tool

# Add a line
dw files prepare /tmp/test.jsonl --add-line '{"custom_id": "req-004", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "test", "messages": [{"role": "user", "content": "Hello"}]}}' --output /tmp/added.jsonl
wc -l /tmp/test.jsonl /tmp/added.jsonl

# Remove lines matching pattern
dw files prepare /tmp/test.jsonl --remove-lines "req-002" --output /tmp/removed.jsonl
wc -l /tmp/test.jsonl /tmp/removed.jsonl
cat /tmp/removed.jsonl
```

## 5. File Upload & Management

```bash
# Upload the test file
dw files upload /tmp/test.jsonl

# Upload with model override (transforms before upload)
dw files upload /tmp/test.jsonl --model "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8"

# List files
dw files list
dw files list --output json

# Get file metadata (use ID from upload output)
dw files get <file-id>

# Download file content
dw files content <file-id>
dw files content <file-id> --output-file /tmp/downloaded.jsonl
diff /tmp/test.jsonl /tmp/downloaded.jsonl

# Cost estimate
dw files cost-estimate <file-id>
dw files cost-estimate <file-id> --completion-window 1h

# Delete (will prompt for confirmation)
dw files delete <file-id>

# Delete without prompt
dw files delete <file-id> --yes
```

## 6. Batch Workflow

```bash
# Upload a file first
dw files upload /tmp/test.jsonl
# Note the file ID from output

# Create a batch
dw batches create --file <file-id> --completion-window 24h

# List batches
dw batches list
dw batches list --active-first

# Get batch details
dw batches get <batch-id>

# Watch progress (polls every 2s, Ctrl+C to stop)
dw batches watch <batch-id>

# After completion — download results
dw batches results <batch-id>
dw batches results <batch-id> --output-file /tmp/results.jsonl
```

## 7. Batch Run (Composite Command)

```bash
# Upload + create in one step
dw batches run /tmp/test.jsonl

# With model override
dw batches run /tmp/test.jsonl --model "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8"

# With watch (blocks until completion)
dw batches run /tmp/test.jsonl --watch

# Run all JSONL files in a directory
mkdir -p /tmp/batch-dir
cp /tmp/test.jsonl /tmp/batch-dir/batch1.jsonl
cp /tmp/test.jsonl /tmp/batch-dir/batch2.jsonl
dw batches run /tmp/batch-dir/
```

## 8. Stream (Zero to Results)

```bash
# Upload + batch + watch + pipe results to stdout
# Progress goes to stderr, results to stdout
dw stream /tmp/test.jsonl

# Save results to file (progress still visible on stderr)
dw stream /tmp/test.jsonl > /tmp/stream-results.jsonl

# With model override
dw stream /tmp/test.jsonl --model "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8"

# With faster completion window
dw stream /tmp/test.jsonl --completion-window 1h
```

## 9. Real-Time Inference

```bash
# Basic prompt (streams tokens)
dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "What is 2+2?"

# With system message
dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "Explain gravity" --system "You are a physics teacher. Be concise."

# With parameters
dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "Write a haiku" --temperature 0.9 --max-tokens 50

# Non-streaming mode
dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "Hello" --no-stream

# Pipe to file
dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "List 5 animals" --output-file /tmp/animals.txt
cat /tmp/animals.txt

# Read prompt from stdin (pipe)
echo "Translate to French: Hello world" | dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8"

# Chain with other commands
echo "Summarize this in one line" | dw realtime "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" > /tmp/summary.txt
```

## 10. Batch Cancel & Retry

```bash
# Create a batch and then cancel it
dw batches run /tmp/test.jsonl
dw batches cancel <batch-id>        # prompts for confirmation
dw batches cancel <batch-id> --yes  # skip prompt

# Retry failed requests in a completed batch
dw batches retry <batch-id>
```

## 11. Examples

```bash
# List available examples
dw examples list

# Clone an example (downloads from GitHub)
dw examples clone synthetic-data-generation
ls synthetic-data-generation/

# Clone to a specific directory
dw examples clone model-evals --dir /tmp/my-evals
ls /tmp/my-evals/
```

## 12. Output Formats

Test each command with all three output modes:

```bash
dw models list --output table
dw models list --output json
dw models list --output plain

dw files list --output table
dw files list --output json
dw files list --output plain

dw batches list --output table
dw batches list --output json

# Verify auto-detection: pipe should default to JSON
dw models list | head -1
# vs terminal should default to table
dw models list
```

## 13. Account Management

```bash
# Current account
dw account current

# List all
dw account list

# The --account flag should override active account (will error if doesn't exist)
dw models list --account nonexistent
```

## 14. Error Handling

```bash
# Invalid API key
dw login --api-key "sk-invalid-key"

# Nonexistent resource
dw files get "file-nonexistent-id"
dw batches get "batch-nonexistent-id"

# Invalid file path
dw files upload /tmp/nonexistent.jsonl
dw batches run /tmp/nonexistent.jsonl

# Missing auth (logout first)
dw logout
dw models list
```

## 15. Shell Completions

```bash
# Generate and test (don't source permanently yet)
dw completions zsh > /tmp/dw-completions.zsh
head -20 /tmp/dw-completions.zsh

dw completions bash > /tmp/dw-completions.bash
head -20 /tmp/dw-completions.bash
```

## 16. Cleanup

```bash
# Re-login for continued use
dw login --api-key "sk-your-key-here"

# Clean up test files
rm -f /tmp/test.jsonl /tmp/bad.jsonl /tmp/missing.jsonl
rm -f /tmp/transformed.jsonl /tmp/params.jsonl /tmp/custom.jsonl
rm -f /tmp/added.jsonl /tmp/removed.jsonl /tmp/downloaded.jsonl
rm -f /tmp/results.jsonl /tmp/stream-results.jsonl
rm -f /tmp/animals.txt /tmp/summary.txt
rm -rf /tmp/batch-dir /tmp/my-evals
rm -rf synthetic-data-generation/

# Remove credentials if done testing
# dw logout --all
```

## Known Limitations (v0.1.0)

- `dw login` (browser flow) requires the control-layer callback endpoint — not yet built
- With `--api-key` login (inference key only), these commands are unavailable:
  - `dw whoami` (requires platform key)
  - `dw models list / get` (requires platform key)
  - `dw webhooks *` (requires platform key)
- `dw realtime` streaming parses the full SSE body at once (not true incremental streaming yet)
- `dw examples clone` requires network access to GitHub
