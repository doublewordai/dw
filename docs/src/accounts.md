# Accounts & Configuration

The CLI supports multiple accounts (personal and org contexts), similar to kubectl contexts.

## Accounts

### Multiple Accounts

Each `dw login` creates a new account. You can log in to multiple accounts and switch between them:

```bash
# Login to personal account
dw login

# Login to an org
dw login --org my-company --as company

# Login to a self-hosted deployment
dw login --as work --server https://dw.internal.example.com
```

### Switching Accounts

```bash
dw account list
dw account switch company
```

### Using a Specific Account for One Command

```bash
dw batches list --account company
```

The `--account` flag overrides the active account for a single command.

### Managing Accounts

```bash
# Rename
dw account rename old-name new-name

# Remove
dw account remove old-account

# Show active
dw account current
```

## Configuration

Configuration is stored in `~/.dw/config.toml`:

```toml
active_account = "you@example.com"

[client]
timeout_secs = 300
connect_timeout_secs = 10
max_retries = 1
poll_interval_secs = 2
```

### Server URLs

For self-hosted or custom deployments:

```bash
# Point both APIs to one server
dw config set-url https://dw.example.com

# Or set individually
dw config set-ai-url https://api.example.com
dw config set-admin-url https://app.example.com

# Reset to defaults
dw config reset-urls
```

### Client Settings

Configure in `~/.dw/config.toml` under the `[client]` section:

| Setting | Default | Description |
|---------|---------|-------------|
| `timeout_secs` | `300` | HTTP request timeout in seconds |
| `connect_timeout_secs` | `10` | TCP connect timeout in seconds |
| `max_retries` | `1` | Max retries on transient errors (0–10) |
| `poll_interval_secs` | `2` | Seconds between polls for `watch` and `stream` (min: 1) |

All fields are optional. Omit individual fields or the entire `[client]` section to use defaults.

### Viewing Configuration

```bash
dw config show
```

Shows active account, default output format, and server URLs.

## Output Formats

The CLI auto-detects the best output format:

| Context | Format | Description |
|---------|--------|-------------|
| Interactive terminal | `table` | Human-readable tables |
| Piped to another command | `json` | One JSON object per line |

Override with the `--output` flag:

```bash
dw batches list --output json
dw models list --output plain
```

Available formats: `table`, `json`, `plain`.
