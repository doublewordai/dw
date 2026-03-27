# Usage & Analytics

## Usage Summary

```bash
dw usage
```

Shows total tokens, requests, and cost broken down by model.

### Date Filtering

```bash
# Usage since a specific date
dw usage --since 2026-03-01

# Usage for a date range
dw usage --since 2026-03-01 --until 2026-03-31
```

Dates can be `YYYY-MM-DD` (date-only) or full RFC 3339 timestamps (`2026-03-01T00:00:00Z`). Date-only values are treated as midnight UTC.

## Recent Requests

```bash
dw requests
```

Lists recent requests with model, status, latency, and token counts.

> **Note:** The `dw requests` command requires the RequestViewer role. Contact your platform administrator if you need access.

### Filtering

```bash
dw requests --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 --since 2026-03-27
```

## Batch Analytics

For per-batch analytics:

```bash
dw batches analytics <batch-id>
```

Shows token usage (prompt, completion, total), average latency, average TTFB, and cost for a completed batch.
