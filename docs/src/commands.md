# Command Reference

Full reference for all `dw` commands. Run `dw <command> --help` for details on any command.

## Auth

| Command | Description |
|---------|-------------|
| `dw login` | Authenticate via browser or API key |
| `dw login --api-key <KEY>` | Headless authentication |
| `dw login --org <ORG>` | Login within an organization |
| `dw login --as <NAME>` | Set a custom account name |
| `dw logout` | Remove active account credentials |
| `dw logout --all` | Remove all stored credentials |
| `dw whoami` | Show authenticated user |

## Accounts

| Command | Description |
|---------|-------------|
| `dw account list` | List all stored accounts |
| `dw account current` | Show the active account |
| `dw account switch <NAME>` | Switch the active account |
| `dw account rename <OLD> <NEW>` | Rename an account |
| `dw account remove <NAME>` | Remove a stored account |

## Models

| Command | Description |
|---------|-------------|
| `dw models list` | List available models |
| `dw models list --type chat` | Filter by type (chat, embeddings, reranker) |
| `dw models get <MODEL>` | Get model details |

## Files

### Remote (requires auth)

| Command | Description |
|---------|-------------|
| `dw files upload <PATH>` | Upload a JSONL file |
| `dw files list` | List uploaded files |
| `dw files list --all` | List all files (auto-paginate) |
| `dw files list --purpose all` | Include output and error files |
| `dw files get <ID>` | Get file metadata |
| `dw files delete <ID>` | Delete a file |
| `dw files content <ID>` | Download file content |
| `dw files cost-estimate <ID>` | Get processing cost estimate |

### Local (no auth needed)

| Command | Description |
|---------|-------------|
| `dw files validate <PATH>` | Validate JSONL format |
| `dw files prepare <PATH>` | Transform JSONL (model, params, images) |
| `dw files stats <PATH>` | Show line count, models, token estimates |
| `dw files sample <PATH> -n <N>` | Random sample from JSONL |
| `dw files merge <FILES...>` | Merge multiple JSONL files |
| `dw files split <PATH>` | Split JSONL into chunks |
| `dw files diff <A> <B>` | Compare two result files |

## Batches

| Command | Description |
|---------|-------------|
| `dw batches run <PATH>` | Upload + create batch (one step) |
| `dw batches run <PATH> --watch` | Upload + create + watch progress |
| `dw batches create --file <ID>` | Create batch from uploaded file |
| `dw batches list` | List batches |
| `dw batches get <ID>` | Get batch details |
| `dw batches cancel <ID>` | Cancel a running batch |
| `dw batches retry <ID>` | Retry failed requests |
| `dw batches results <IDS...>` | Download results (or `--from-file`) |
| `dw batches watch <IDS...>` | Watch batch progress |
| `dw batches analytics <IDS...>` | Show batch analytics (or `--from-file`) |

## Stream

| Command | Description |
|---------|-------------|
| `dw stream <PATH>` | Upload, batch, watch, and pipe results |

## Realtime

| Command | Description |
|---------|-------------|
| `dw realtime <MODEL> <PROMPT>` | One-shot streaming inference |

## Usage & Requests

| Command | Description |
|---------|-------------|
| `dw usage` | Show usage summary (tokens, cost, requests) |
| `dw usage --since 2026-03-01` | Usage from a specific date |
| `dw usage --since 2026-03-01 --until 2026-03-31` | Usage for a date range |
| `dw requests` | List recent requests |

## Keys

| Command | Description |
|---------|-------------|
| `dw keys create --name <NAME>` | Create an API key |
| `dw keys list` | List API keys (secrets masked) |
| `dw keys delete <ID>` | Delete an API key |

## Webhooks

| Command | Description |
|---------|-------------|
| `dw webhooks create --url <URL>` | Create a webhook |
| `dw webhooks list` | List webhooks |
| `dw webhooks delete <ID>` | Delete a webhook |
| `dw webhooks rotate-secret <ID>` | Rotate signing secret |

## Projects

| Command | Description |
|---------|-------------|
| `dw project init [NAME]` | Create a new project from template |
| `dw project setup` | Run project setup command |
| `dw project run <STEP>` | Run a named step |
| `dw project run-all` | Run full workflow |
| `dw project run-all --continue` | Resume from last completed step |
| `dw project status` | Show run progress |
| `dw project clean` | Remove artifacts |
| `dw project info` | Show steps and workflow |

## Examples

| Command | Description |
|---------|-------------|
| `dw examples list` | List available examples |
| `dw examples clone <NAME>` | Clone an example project |

## Config

| Command | Description |
|---------|-------------|
| `dw config show` | Show current configuration |
| `dw config set-url <URL>` | Set both API URLs |
| `dw config set-ai-url <URL>` | Set inference API URL |
| `dw config set-admin-url <URL>` | Set admin API URL |
| `dw config reset-urls` | Reset URLs to defaults |

## Other

| Command | Description |
|---------|-------------|
| `dw update` | Self-update to latest release |
| `dw completions <SHELL>` | Generate shell completions |
