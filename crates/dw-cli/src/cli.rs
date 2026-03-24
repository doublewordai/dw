use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::output::OutputFormat;

const AFTER_HELP: &str = "\
Quick start:
  dw login                                  Authenticate via browser
  dw login --api-key <KEY>                  Authenticate with API key
  dw models list                             See available models
  dw stream batch.jsonl > results.jsonl      Run and stream results
  dw realtime <model> \"your prompt\"          One-shot inference

Run 'dw <command> --help' for details on any command.
Docs: https://github.com/doublewordai/dw";

#[derive(Parser)]
#[command(
    name = "dw",
    about = "Doubleword Batch Inference CLI",
    long_about = "Doubleword Batch Inference CLI\n\n\
        Upload JSONL files, run batches, stream results, and send \
        real-time inference requests — all from the terminal.",
    version,
    after_help = AFTER_HELP,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format (table, json, plain). Auto-detects: table for TTY, json for pipes.
    #[arg(long, global = true)]
    pub output: Option<OutputFormat>,

    /// Account context override (use instead of active account).
    #[arg(long, global = true)]
    pub account: Option<String>,

    /// Override both server URLs (inference + admin) to point to a single host.
    #[arg(long, global = true)]
    pub server: Option<String>,

    /// Override inference API server URL (default: api.doubleword.ai).
    #[arg(long, global = true)]
    pub server_ai: Option<String>,

    /// Override admin API server URL (default: app.doubleword.ai).
    #[arg(long, global = true)]
    pub server_admin: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authenticate via browser or API key.
    Login(LoginArgs),

    /// Remove stored credentials.
    Logout(LogoutArgs),

    /// Show the currently authenticated user.
    Whoami,

    /// Manage accounts (personal and org contexts).
    #[command(subcommand)]
    Account(AccountCommands),

    /// List and inspect available models.
    #[command(subcommand)]
    Models(ModelCommands),

    /// Upload, manage, and prepare JSONL batch files.
    #[command(subcommand)]
    Files(FileCommands),

    /// Create, monitor, and manage batch jobs.
    #[command(subcommand)]
    Batches(BatchCommands),

    /// Upload, run, and stream batch results in one command.
    Stream(StreamArgs),

    /// Send a real-time inference request.
    Realtime(RealtimeArgs),

    /// Show usage summary (tokens, cost, requests by model).
    Usage(UsageArgs),

    /// List recent requests (requires RequestViewer role).
    Requests(RequestsArgs),

    /// Manage batch completion webhooks.
    #[command(subcommand)]
    Webhooks(WebhookCommands),

    /// Browse and clone use-case examples.
    #[command(subcommand)]
    Examples(ExampleCommands),

    /// View and update CLI configuration.
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Generate shell completions.
    Completions(CompletionsArgs),
}

// --- Login / Logout ---

#[derive(clap::Args)]
pub struct LoginArgs {
    /// Authenticate with an API key instead of browser flow.
    #[arg(long)]
    pub api_key: Option<String>,

    /// Login within an organization context.
    #[arg(long)]
    pub org: Option<String>,

    /// Custom name for this account (overrides auto-generated name).
    #[arg(long, value_name = "NAME")]
    pub r#as: Option<String>,
}

#[derive(clap::Args)]
pub struct LogoutArgs {
    /// Account to log out of. Defaults to active account.
    pub account: Option<String>,

    /// Log out of all accounts.
    #[arg(long)]
    pub all: bool,
}

// --- Config ---

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration.
    Show,
    /// Set the server URL for both inference and admin APIs.
    SetUrl {
        /// URL to use for both APIs (e.g. https://staging.doubleword.ai).
        url: String,
    },
    /// Set the inference API server URL.
    SetAiUrl {
        /// Inference API URL (e.g. https://api.doubleword.ai).
        url: String,
    },
    /// Set the admin API server URL.
    SetAdminUrl {
        /// Admin API URL (e.g. https://app.doubleword.ai).
        url: String,
    },
    /// Reset server URLs to defaults.
    ResetUrls,
}

// --- Account ---

#[derive(Subcommand)]
pub enum AccountCommands {
    /// List all stored accounts.
    List,
    /// Switch the active account.
    Switch {
        /// Account name to switch to.
        name: String,
    },
    /// Show the currently active account.
    Current,
    /// Rename an account.
    Rename {
        /// Current account name.
        current: String,
        /// New name for the account.
        new: String,
    },
    /// Remove a stored account.
    Remove {
        /// Account name to remove.
        name: String,
    },
}

// --- Models ---

#[derive(Subcommand)]
pub enum ModelCommands {
    /// List available models.
    List {
        /// Filter by model type (chat, embeddings, reranker).
        #[arg(long, short = 't')]
        r#type: Option<String>,
    },
    /// Get details for a specific model.
    Get {
        /// Model ID or alias.
        model: String,
    },
}

// --- Files ---

#[derive(Subcommand)]
pub enum FileCommands {
    /// Upload a JSONL file for batch processing.
    Upload(FileUploadArgs),
    /// List uploaded files. Shows input files (purpose=batch) by default.
    List {
        /// Maximum number of files to return (default: 20, max: 100).
        #[arg(long, short = 'n', default_value = "20")]
        limit: i64,
        /// Cursor: show files after this file ID (for pagination).
        #[arg(long)]
        after: Option<String>,
        /// Fetch all files (auto-paginate). Ignores --limit and --after.
        #[arg(long)]
        all: bool,
        /// Filter by purpose: batch (default), batch_output, batch_error, or all.
        #[arg(long, default_value = "batch")]
        purpose: String,
    },
    /// Get file metadata.
    Get {
        /// File ID.
        id: String,
    },
    /// Delete a file.
    Delete {
        /// File ID.
        id: String,
        /// Skip confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Download file content.
    Content {
        /// File ID.
        id: String,
        /// Write to file instead of stdout.
        #[arg(long, short = 'o')]
        output_file: Option<PathBuf>,
    },
    /// Get estimated cost for processing a file.
    CostEstimate {
        /// File ID.
        id: String,
        /// Completion window (e.g. "24h", "1h").
        #[arg(long, short = 'w')]
        completion_window: Option<String>,
    },
    /// Validate a local JSONL file without uploading.
    Validate {
        /// Path to JSONL file.
        path: PathBuf,
    },
    /// Transform a local JSONL file (model override, params, image encoding).
    Prepare(FilePrepareArgs),
}

#[derive(clap::Args)]
pub struct FileUploadArgs {
    /// Path to JSONL file.
    pub path: PathBuf,

    /// Override the model on every line.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Set temperature on every line.
    #[arg(long)]
    pub temperature: Option<f64>,

    /// Set max_tokens on every line.
    #[arg(long)]
    pub max_tokens: Option<u64>,

    /// Encode local images and URLs to base64 data URIs.
    #[arg(long)]
    pub encode_images: bool,
}

#[derive(clap::Args)]
pub struct FilePrepareArgs {
    /// Path to JSONL file or directory of .jsonl files.
    pub path: PathBuf,

    /// Override the model on every line.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Set temperature on every line.
    #[arg(long)]
    pub temperature: Option<f64>,

    /// Set max_tokens on every line.
    #[arg(long)]
    pub max_tokens: Option<u64>,

    /// Set top_p on every line.
    #[arg(long)]
    pub top_p: Option<f64>,

    /// Set an arbitrary key=value in the body (dot-notation, e.g. body.stream=false).
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub set_fields: Vec<String>,

    /// Append a JSON line to the file.
    #[arg(long = "add-line")]
    pub add_lines: Vec<String>,

    /// Remove lines where custom_id matches this pattern.
    #[arg(long = "remove-lines")]
    pub remove_lines: Option<String>,

    /// Encode local images and URLs to base64 data URIs.
    #[arg(long)]
    pub encode_images: bool,

    /// Output path (default: overwrite in place).
    #[arg(long = "output-file", short = 'o')]
    pub output_file: Option<PathBuf>,
}

// --- Batches ---

#[derive(Subcommand)]
pub enum BatchCommands {
    /// Create a batch from an uploaded file.
    Create(BatchCreateArgs),
    /// List batches.
    List {
        /// Maximum number of batches to return (default: 20, max: 100).
        #[arg(long, short = 'n', default_value = "20")]
        limit: i64,
        /// Cursor: show batches after this batch ID (for pagination).
        #[arg(long)]
        after: Option<String>,
        /// Fetch all batches (auto-paginate). Ignores --limit and --after.
        #[arg(long)]
        all: bool,
        /// Show active batches first.
        #[arg(long)]
        active_first: bool,
    },
    /// Get batch details.
    Get {
        /// Batch ID.
        id: String,
    },
    /// Cancel a running batch.
    Cancel {
        /// Batch ID.
        id: String,
        /// Skip confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Retry failed requests in a batch.
    Retry {
        /// Batch ID.
        id: String,
    },
    /// Download batch results.
    Results {
        /// Batch ID.
        id: String,
        /// Write to file instead of stdout.
        #[arg(long, short = 'o')]
        output_file: Option<PathBuf>,
    },
    /// Upload and create a batch in one step.
    Run(BatchRunArgs),
    /// Watch one or more batches' progress until completion.
    Watch {
        /// Batch ID(s).
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// Show analytics for a batch (tokens, latency, cost).
    Analytics {
        /// Batch ID.
        id: String,
    },
}

#[derive(clap::Args)]
pub struct BatchCreateArgs {
    /// File ID of the uploaded JSONL file.
    #[arg(long)]
    pub file: String,

    /// Completion window (e.g. "24h", "1h").
    #[arg(long, short = 'w', default_value = "24h")]
    pub completion_window: String,

    /// Metadata key=value pairs.
    #[arg(long = "metadata", value_name = "KEY=VALUE")]
    pub metadata: Vec<String>,
}

#[derive(clap::Args)]
pub struct BatchRunArgs {
    /// Path to JSONL file or directory of JSONL files.
    pub path: PathBuf,

    /// Override the model on every line.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Completion window (e.g. "24h", "1h").
    #[arg(long, short = 'w', default_value = "24h")]
    pub completion_window: String,

    /// Watch progress after creating the batch.
    #[arg(long)]
    pub watch: bool,
}

// --- Stream ---

#[derive(clap::Args)]
pub struct StreamArgs {
    /// Path to JSONL file or directory of JSONL files.
    pub path: PathBuf,

    /// Override the model on every line.
    #[arg(long, short = 'm')]
    pub model: Option<String>,

    /// Completion window (e.g. "24h", "1h").
    #[arg(long, short = 'w', default_value = "24h")]
    pub completion_window: String,
}

// --- Realtime ---

#[derive(clap::Args)]
pub struct RealtimeArgs {
    /// Model to use (alias or full name).
    pub model: String,

    /// Prompt text. If omitted and stdin is not a TTY, reads from stdin.
    pub prompt: Option<String>,

    /// System message.
    #[arg(long)]
    pub system: Option<String>,

    /// Maximum tokens to generate.
    #[arg(long)]
    pub max_tokens: Option<u64>,

    /// Sampling temperature.
    #[arg(long)]
    pub temperature: Option<f64>,

    /// Disable streaming (wait for full response).
    #[arg(long)]
    pub no_stream: bool,

    /// Write output to file instead of stdout.
    #[arg(long, short = 'o')]
    pub output_file: Option<PathBuf>,

    /// Print token usage summary after completion.
    #[arg(long)]
    pub usage: bool,
}

// --- Webhooks ---

#[derive(Subcommand)]
pub enum WebhookCommands {
    /// Create a webhook.
    Create {
        /// Webhook URL (must be HTTPS).
        #[arg(long)]
        url: String,
        /// Event types to subscribe to (comma-separated).
        #[arg(long)]
        events: Option<String>,
        /// Description.
        #[arg(long)]
        description: Option<String>,
    },
    /// List webhooks.
    List,
    /// Delete a webhook.
    Delete {
        /// Webhook ID.
        id: String,
        /// Skip confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Rotate a webhook's signing secret.
    RotateSecret {
        /// Webhook ID.
        id: String,
    },
}

// --- Examples ---

#[derive(Subcommand)]
pub enum ExampleCommands {
    /// List available use-case examples.
    List,
    /// Clone a use-case example into the current directory.
    Clone {
        /// Example name (e.g. "synthetic-data-generation").
        name: String,
        /// Target directory.
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
}

// --- Completions ---

#[derive(clap::Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    pub shell: clap_complete::Shell,
}

#[derive(clap::Args)]
pub struct UsageArgs {
    /// Start date (ISO 8601, e.g. 2026-03-01). Without dates, shows all-time usage.
    #[arg(long)]
    pub since: Option<String>,
    /// End date (ISO 8601, e.g. 2026-03-31).
    #[arg(long)]
    pub until: Option<String>,
}

#[derive(clap::Args)]
pub struct RequestsArgs {
    /// Maximum number of requests to return (default: 20, max: 100).
    #[arg(long, short = 'n', default_value = "20")]
    pub limit: i64,
    /// Number of entries to skip (for pagination).
    #[arg(long, default_value = "0")]
    pub skip: i64,
    /// Filter by model name.
    #[arg(long, short = 'm')]
    pub model: Option<String>,
    /// Filter: requests after this date (ISO 8601, e.g. 2026-03-01).
    #[arg(long)]
    pub since: Option<String>,
    /// Filter: requests before this date (ISO 8601).
    #[arg(long)]
    pub until: Option<String>,
    /// Filter by batch ID.
    #[arg(long)]
    pub batch_id: Option<String>,
    /// Filter by HTTP status code.
    #[arg(long)]
    pub status: Option<i32>,
}
