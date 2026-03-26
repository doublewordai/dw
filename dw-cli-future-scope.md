# DW CLI — Future Scope & Deferred Ideas

Items consciously deferred from v1. Revisit after initial release and internal testing.

---

## Device Code Flow for Remote/Headless Login
- Current `dw login` uses localhost redirect — works on local machines but not over SSH or in containers
- Device Code Flow (RFC 8628) shows a code + URL, user authenticates in any browser on any device
- Used by GitHub CLI, AWS SSO, Azure CLI, Vercel CLI, OpenAI CLI
- Requires identity provider support (oauth2-proxy/Google OAuth need to support the device authorization grant)
- Would make `dw login` work in SSH sessions, containers, WSL, cloud shells
- Until then, remote users use `dw login --api-key`
- Also consider: manual link copy with confirmation (show URL, user pastes into remote browser, redirect still works if port is forwarded)

## CLI Key Management & Eviction
- Cap active CLI keys per user (e.g. 10) — new logins soft-delete the oldest CLI key pair (FIFO)
- Keeps the key count bounded without requiring users to manually manage internal keys
- Users never see CLI keys in the dashboard — they're created/evicted automatically by `dw login`
- Prevents key spam from repeated logins while allowing multiple machines/sessions
- Consider also: key expiry (TTL, e.g. 30 days), auto-rotation on expiry

## Streams as Persistent Construct
- Reusable stream endpoints that accept ongoing request submissions
- Auto-batching: submit individual requests, system batches them automatically
- Consuming from multiple sources / chaining streams
- `dw stream create` → persistent stream ID, `dw stream push` → add requests
- Compositional piping: `dw stream start | dw stream consume`
- Semantics discussion: eager (as-fast-as-possible) vs continuous pipeline

## API Key Management via CLI
- `dw keys create` / `dw keys list` / `dw keys delete`
- Deferred because CLI manages its own keys internally (login/logout)
- Users who need to create keys for agents/CI can use the dashboard
- Risk of confusion between CLI-managed keys and user-created keys
- Revisit if there's demand for headless key management workflows

## TUI Views (ratatui)
- Interactive batch monitoring with live progress bars
- File browser with JSONL inspection
- Model playground with streaming responses
- Settings management (keys, webhooks, account)
- Consider as `--tui` flag or `dw tui` subcommand, not the default happy path
- Only invest if there's evidence of user demand

## Analytics & Usage Commands
- `dw usage` — token/request/cost summary by model, time period
- `dw requests list` — recent requests with model, status, latency, cost
- `dw requests aggregate` — aggregate analytics with grouping
- Cost reports (useful for agents generating spend summaries)
- Be cautious: easy API access to analytics could enable high-poll-rate upstream dashboards

## Self-Update
- `dw update` — download and replace binary with latest release
- Version check on startup (non-blocking, once per day)
- Consider: should this be opt-in? Some orgs want pinned versions

## Native Library Bindings (FFI)
- PyO3 bindings for `dw-client` → `from dw import Client`
- napi-rs bindings for Node.js → `const { Client } = require('dw')`
- Enables programmatic use in scripts, notebooks, CI pipelines
- Architecture already supports this: `dw-client` has no CLI dependencies

## API Key Expiry, Rotation & Re-Auth
- TTL on CLI keys (configurable, default ~30 days)
- Auto-rotation: CLI detects expiring key, re-authenticates transparently
- Notification: warn user N days before expiry
- Server-side: add `expires_at` column to `api_keys` table
- Token refresh / re-auth prompting when session expires mid-operation
- Detect 401 mid-workflow and prompt for `dw login` instead of just failing

## Project Scaffolding (`dw init`)
- `dw init` — create project structure (pyproject.toml, sample JSONL, .env template)
- Templates for common patterns: single batch, multi-stage pipeline, embedding job
- Integrates with `dw examples` for seeding from use-cases

## Advanced Pipe Composition
- `dw stream start -f input.jsonl | dw stream consume` — dispatch and stream results
- Chaining batches: output of batch A feeds as input to batch B
- `jq` integration patterns for transforming between stages
- Presigned S3 URLs for large file streaming
- Directory watching: `dw watch ./input/ --submit-on-change`

## Webhook Enhancements
- More event types beyond batch.completed/failed
- `dw webhooks test <id>` — send test payload to verify endpoint
- Webhook logs: `dw webhooks logs <id>` — recent delivery attempts and responses
- Retry configuration per webhook

## Multi-Run Project State
- Named/numbered runs (`runs/run-001/`, `runs/run-002/`) instead of single overwriting state
- `dw project run-all --name experiment-a` to label runs
- `dw project status --run run-001` to inspect specific runs
- `dw project diff run-001 run-002` to compare results across runs
- Run history and statistics
- Symlink `runs/latest` to most recent run

## Multi-File Batch Orchestration
- Dependency graphs between batches (batch B waits for batch A)
- Sequential pipeline execution from a manifest file
- `dw pipeline run pipeline.yaml` ��� declarative multi-stage execution
- DAG visualization in TUI

## CLI Configuration Management
- `dw config set default-model 30b`
- `dw config set default-completion-window 1h`
- `dw config set output-format json`
- Per-account config overrides
- Environment variable overrides for CI
- Advanced client configuration: request timeouts, retry count, retry backoff strategy
- Configurable polling intervals for `watch` and `stream`

## Request & Usage Monitoring
- `dw requests list` — list recent requests with model, status, latency
- `dw requests list --model X --since 2h` — filtered views
- `dw usage` — show usage summary (tokens, requests, cost by model, time period)
- Be cautious about enabling high-poll-rate consumption of analytics data

## Webhook Secret Rotation
- `dw webhooks rotate-secret <webhook_id>` — rotate signing secret via CLI
- Currently webhooks can be created/listed/deleted but secret rotation is not exposed

## Installation & Distribution Polish
- Host install script on doubleword.ai landing page (`curl -fsSL https://doubleword.ai/install.sh | sh`)
- Landing page integration: "Download or use web console" with prominent install command
- Link to CLI documentation from landing page
- `dw update` — self-update binary to latest release
- First-run experience: ASCII art banner, welcome message with quick-start steps when no credentials found
- Verify install.sh against real GitHub releases (currently functional but needs live release to test curl flow)
- Static linking where possible for maximum portability
- CI-friendly: must work non-interactive, no browser, API key only

## Apple Code Signing & Notarization
- Sign macOS binaries to avoid Gatekeeper warnings
- Requires Apple Developer account ($99/year)
- Automated via CI with stored credentials
- Without this, macOS users must right-click → Open or `xattr -d com.apple.quarantine`

## Homebrew Distribution
- `brew install doubleword/tap/dw`
- Homebrew formula in a dedicated tap repo
- Auto-updated on release via CI

## JSONL Result-to-Input Transformation / Templating
- Transform batch result JSONL into new input JSONL for multi-stage pipelines
- Template system: define how to extract content from results and build new prompts
- Example: stage 1 outputs scenarios → template builds stage 2 conversation prompts from them
- Could use Handlebars/Tera templates or a custom DSL
- Enables fully CLI-native multi-stage pipelines without Python between stages
- Related: parfold-style LLM primitives (summarize, search, sort) as CLI operations
- `dw files transform results.jsonl --template stage2.toml -o stage2-input.jsonl`

## Declarative Pipeline Execution
- `dw pipeline run pipeline.toml` — sequential multi-stage batch execution from manifest
- Each stage: optional prepare command → `dw stream` → pass results to next stage
- `{prev_results}` variable substitution between stages
- Progress tracking across the full pipeline
- DAG support for parallel stages with dependencies

## Batch Analytics (Post-Completion)
- `dw batches analytics <id>` — latency distribution, error rates, throughput
- Export analytics as CSV/JSON for external dashboards
- Comparative analytics across batches

## Real-Time Request Logging
- `dw requests tail` — live stream of requests as they complete
- Filtering by model, status, batch
- Useful for debugging and monitoring

## Multi-Environment Support
- `dw env add staging --server staging.doubleword.ai`
- `dw env switch staging`
- Per-environment credentials and config
- Useful for internal testing against staging
