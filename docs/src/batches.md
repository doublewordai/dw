# Batch Processing

Batch processing lets you submit thousands of API requests in a single JSONL file and process them asynchronously at reduced cost.

## Creating a Batch

### One-Step: `dw batches run`

Upload a file and create a batch in one command:

```bash
dw batches run batch.jsonl
```

With progress watching:

```bash
dw batches run batch.jsonl --watch
```

Override the model for all requests:

```bash
dw batches run batch.jsonl --model Qwen/Qwen3-VL-235B-A22B-Instruct-FP8 --watch
```

Process an entire directory of JSONL files:

```bash
dw batches run output/ --watch
```

### Step-by-Step

```bash
# Upload the file
dw files upload batch.jsonl

# Create a batch from the uploaded file
dw batches create --file file-abc123

# Optionally set a 1-hour completion window (default: 24h)
dw batches create --file file-abc123 --completion-window 1h

# Add metadata
dw batches create --file file-abc123 --metadata project=evals --metadata run=1
```

## Monitoring

### Watch Progress

Watch one batch:

```bash
dw batches watch batch-abc123
```

Watch multiple batches with parallel progress bars:

```bash
dw batches watch batch-abc123 batch-def456 batch-ghi789
```

### Check Status

```bash
dw batches get batch-abc123
```

### List Batches

```bash
# Recent batches
dw batches list

# Active batches first
dw batches list --active-first

# All batches (auto-paginate)
dw batches list --all
```

## Results

### Download Results

```bash
dw batches results batch-abc123 -o results.jsonl
```

Without `-o`, results are printed to stdout. Multiple IDs can be passed to combine results:

```bash
dw batches results id1 id2 id3 -o results.jsonl
dw batches results --from-file .batch-id -o results.jsonl
```

### Batch Analytics

```bash
dw batches analytics batch-abc123
```

Shows token usage, latency breakdown, and cost for a completed batch. Supports multiple IDs:

```bash
dw batches analytics id1 id2
dw batches analytics --from-file .batch-id
```

## Cancellation and Retry

### Cancel a Batch

```bash
dw batches cancel batch-abc123
```

Prompts for confirmation. Use `--yes` to skip:

```bash
dw batches cancel batch-abc123 --yes
```

### Retry Failed Requests

```bash
dw batches retry batch-abc123
```

Creates a new batch containing only the failed requests from the original batch.

## Completion Windows

The completion window determines the SLA for batch processing:

| Window | Description |
|--------|-------------|
| `24h` | Default. Standard batch pricing. |
| `1h` | Priority processing. Results in ~1 hour. |

```bash
dw batches run batch.jsonl --completion-window 1h
```

## Cost Estimates

Before submitting, check the estimated cost:

```bash
dw files upload batch.jsonl
dw files cost-estimate <file-id>
dw files cost-estimate <file-id> --completion-window 1h
```

## Saving Batch IDs

When running multiple files, save batch IDs for later reference:

```bash
dw batches run output/ --output-id batch-ids.txt
```

This writes one batch ID per line to the specified file.
