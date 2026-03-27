# Streaming Results

`dw stream` is the fastest path from a JSONL file to results. It uploads the file, creates a batch, watches progress, and streams results to stdout as they complete.

## Basic Usage

```bash
dw stream batch.jsonl > results.jsonl
```

Progress is printed to stderr, results to stdout. This means you can pipe results while still seeing progress.

## Streaming a Directory

When given a directory, `dw stream` processes all `.jsonl` files in it:

```bash
dw stream output/ > results.jsonl
```

Each file becomes a separate batch. Progress bars for all batches are shown in parallel.

## Model Override

Set the model for all requests without modifying the file:

```bash
dw stream batch.jsonl --model Qwen/Qwen3-VL-235B-A22B-Instruct-FP8 > results.jsonl
```

## Completion Window

```bash
dw stream batch.jsonl --completion-window 1h > results.jsonl
```

## How It Works

1. If `--model` is specified, the file is transformed to a temp file with the model set
2. The file is uploaded via `POST /v1/files`
3. A batch is created from the uploaded file
4. The CLI polls the batch status, showing a progress bar
5. As results become available, they're fetched via the file content endpoint and written to stdout
6. Polling continues until the batch completes

Results are streamed incrementally — you don't have to wait for the entire batch to finish before seeing output.

## When to Use Stream vs Batches Run

| Use Case | Command |
|----------|---------|
| Quick one-off, pipe to file | `dw stream batch.jsonl > results.jsonl` |
| Fire and forget, check later | `dw batches run batch.jsonl` |
| Multiple files, watch progress | `dw batches run output/ --watch` |
| CI pipeline, save batch ID | `dw batches run batch.jsonl --output-id batch.txt` |

`dw stream` is a convenience wrapper. For more control (metadata, retries, ID tracking), use `dw batches run` and `dw batches results` separately.
