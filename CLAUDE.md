# DW CLI — Codebase Guide

## Overview

`dw` is the Doubleword Batch Inference CLI — a terminal tool for developers and agents to interact with the Doubleword platform. It wraps the Doubleword API with ergonomic commands for file management, batch processing, real-time inference, and JSONL manipulation.

## Architecture

Two crates in a Cargo workspace:

- **`dw-client`** (`crates/dw-client/`) — Pure API client library. No CLI dependencies. Wraps `reqwest` calls to the Doubleword API with typed request/response structs.
  - `client.rs` — HTTP client with dual API surface support (AI + Admin)
  - `types/` — Request/response types mirroring the Doubleword API
  - `endpoints/` — One file per API resource (files, batches, models, etc.)
  - `error.rs` — API error parsing and typed errors

- **`dw-cli`** (`crates/dw-cli/`) — Binary crate. CLI commands built on top of `dw-client`.
  - `cli.rs` — clap definitions for all commands and flags
  - `config.rs` — `~/.dw/` config and credentials management
  - `output.rs` — Table/JSON/plain output formatting
  - `commands/` — One file per command group (batches, files, auth, etc.)
  - `jsonl/` — Local JSONL validation, transformation, image encoding

## API Surfaces

The CLI talks to two API surfaces:

| Surface | Base URL | API Key | Endpoints |
|---------|----------|---------|-----------|
| AI | `api.doubleword.ai` | Realtime key | Files, batches, models, inference |
| Admin | `app.doubleword.ai` | Platform key | Webhooks, whoami, orgs, API keys |

The `DwClient` handles routing to the correct surface automatically.

## Auth Model

- **Browser login** (`dw login`): Opens browser → SSO → server creates two external API keys (inference + platform) → redirects to CLI's localhost server with keys
- **Headless login** (`dw login --api-key`): Stores an inference key directly (limited functionality)
- **Credentials**: Stored in `~/.dw/credentials.toml` (0600 permissions)
- **Accounts**: kubectl-style contexts. Account key = display name (what you see is what you type). `dw account switch`, `dw account rename`, `dw account remove` manage them. `--as` flag on login overrides the auto-generated name.

External keys != hidden keys. External keys authenticate and scope paths. Hidden keys are static internal keys for billing attribution — managed automatically by the server.

## Key Commands

| Command | What it does |
|---------|-------------|
| `dw stream <path>` | Upload + create batch + watch + pipe results (zero to results) |
| `dw batches run <path>` | Upload + create batch (with optional --watch) |
| `dw realtime <model> "prompt"` | One-shot streaming inference |
| `dw files prepare <path>` | Local JSONL manipulation (model override, params, image encoding) |

## Development

```bash
just build    # Build both crates
just test     # Run tests
just lint     # Clippy + fmt check
just run -- batches list  # Run CLI with args
```

## Testing

- Unit tests: `cargo test`
- Manual testing: `just run -- <command>`
- The binary is at `target/debug/dw`

## Conventions

- All user-visible output goes to stderr (eprintln!). Only data (results, file content) goes to stdout.
- Use `OutputFormat` for all list/detail commands (table for TTY, json for pipes).
- Commands that modify state (delete, cancel) prompt for confirmation unless `--yes` is passed.
