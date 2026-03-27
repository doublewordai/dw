# Global Flags

These flags are available on every `dw` command.

| Flag | Description |
|------|-------------|
| `--output <FORMAT>` | Output format: `table`, `json`, or `plain`. Auto-detected from TTY by default. |
| `--account <NAME>` | Use a specific account instead of the active one. |
| `--server <URL>` | Override both inference and admin API URLs. |
| `--server-ai <URL>` | Override the inference API URL (default: `https://api.doubleword.ai`). |
| `--server-admin <URL>` | Override the admin API URL (default: `https://app.doubleword.ai`). |
| `--help` | Show help for any command. |
| `--version` | Show the CLI version. |

## Output Format

The CLI auto-detects the output format based on whether stdout is a TTY:

- **TTY (interactive terminal):** `table` — human-readable tables with headers and alignment
- **Pipe or redirect:** `json` — JSON output suitable for `jq` processing (NDJSON for lists, pretty-printed for single items)

Override with `--output`:

```bash
# Force JSON in terminal
dw batches list --output json

# Force table when piping (not common, but possible)
dw batches list --output table | less

# Plain text (tab-separated, no headers)
dw batches list --output plain
```

## Server Overrides

Useful for self-hosted or custom deployments:

```bash
# Both APIs on the same server
dw batches list --server https://dw.example.com

# Different URLs for inference and admin
dw stream batch.jsonl --server-ai https://api.example.com --server-admin https://app.example.com
```

For persistent server overrides, use `dw config set-url` instead. See [Accounts & Configuration](accounts.md).

## Account Override

Run a command as a different account without switching:

```bash
dw batches list --account my-org
dw models list --account company
```
