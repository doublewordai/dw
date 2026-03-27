# Quickstart

This guide walks through a complete batch workflow: prepare a file, submit it, and get results.

## 1. Log In

```bash
dw login
```

## 2. See Available Models

```bash
dw models list
```

## 3. Create a Batch File

A batch file is JSONL where each line is an API request:

```json
{"custom_id": "q1", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8", "messages": [{"role": "user", "content": "What is batch inference?"}], "max_tokens": 256}}
{"custom_id": "q2", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8", "messages": [{"role": "user", "content": "Explain transformers in 2 sentences."}], "max_tokens": 256}}
```

Save this as `batch.jsonl`.

## 4. Validate and Inspect

```bash
dw files validate batch.jsonl
dw files stats batch.jsonl
```

## 5. Submit and Stream Results

The fastest path from file to results:

```bash
dw stream batch.jsonl > results.jsonl
```

This uploads the file, creates a batch, watches progress, and pipes results to stdout as they complete.

## 6. Check Cost

`dw stream` prints `Batch: <id>` to stderr when the batch is created. Use that ID to see the cost breakdown:

```bash
dw batches analytics <batch-id>
```

## Alternative: Step-by-Step

If you prefer manual control over each step:

```bash
# Upload
dw files upload batch.jsonl

# Create batch from the uploaded file
dw batches create --file <file-id>

# Watch progress
dw batches watch <batch-id>

# Download results
dw batches results <batch-id> -o results.jsonl
```

## Real-Time Inference

For one-off requests without a batch file:

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "What is batch inference?"
```

## Next Steps

- [Batch Processing](batches.md) — full batch workflow details
- [Local File Operations](file-tools.md) — validate, prepare, merge, split JSONL
- [Project System](projects.md) — scaffold and run multi-step projects
- [Examples](examples.md) — real-world use-case examples
