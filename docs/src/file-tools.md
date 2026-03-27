# Local File Operations

The CLI includes local JSONL tools that run without uploading or authenticating. These are useful for preparing, inspecting, and manipulating batch files before submission.

## Validate

Check a JSONL file for format errors:

```bash
dw files validate batch.jsonl
```

Validates that each line is valid JSON with the required fields (`custom_id`, `method`, `url`, `body`).

## Stats

Show statistics for a JSONL file:

```bash
dw files stats batch.jsonl
```

Output includes line count, models used, and estimated token counts.

## Prepare

Transform a JSONL file in place (or to a new file):

```bash
# Set the model on every line
dw files prepare batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8

# Set model and temperature
dw files prepare batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 --temperature 0

# Write to a new file instead of modifying in place
dw files prepare batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 -o batch-30b.jsonl

# Prepare all files in a directory
dw files prepare output/ --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8
```

### Setting Arbitrary Fields

Use `--set` for any field in the request body:

```bash
dw files prepare batch.jsonl --set body.stream=false --set body.top_p=0.9
```

### Encoding Images

Convert local image paths and URLs to base64 data URIs:

```bash
dw files prepare batch.jsonl --encode-images
```

### Adding and Removing Lines

```bash
# Append a new request line
dw files prepare batch.jsonl --add-line '{"custom_id":"extra","method":"POST","url":"/v1/chat/completions","body":{"messages":[{"role":"user","content":"test"}]}}'

# Remove lines matching a pattern
dw files prepare batch.jsonl --remove-lines "test-*"
```

## Sample

Extract a random sample from a JSONL file:

```bash
dw files sample batch.jsonl -n 10 -o sample.jsonl
```

Uses reservoir sampling for bounded memory on large files.

## Merge

Combine multiple JSONL files into one:

```bash
dw files merge file1.jsonl file2.jsonl file3.jsonl -o merged.jsonl
```

## Split

Split a JSONL file into chunks:

```bash
dw files split large.jsonl --chunk-size 1000
```

Creates `large_001.jsonl`, `large_002.jsonl`, etc. in the same directory. Use `-o` for a different output directory:

```bash
dw files split large.jsonl --chunk-size 1000 -o chunks/
```

## Diff

Compare two JSONL result files by `custom_id`:

```bash
dw files diff results-30b.jsonl results-235b.jsonl
```

Shows which custom_ids are present in one file but not the other, and a content hash comparison for matching IDs.
