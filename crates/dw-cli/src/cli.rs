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

    /// Create, list, and delete API keys.
    #[command(subcommand)]
    Keys(KeyCommands),

    /// Manage batch completion webhooks.
    #[command(subcommand)]
    Webhooks(WebhookCommands),

    /// Browse and clone use-case examples.
    #[command(subcommand)]
    Examples(ExampleCommands),

    /// Run project steps defined in dw.toml.
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Update dw to the latest release.
    Update,

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
    /// Show stats for a local JSONL file (line count, models, estimated tokens).
    Stats {
        /// Path to JSONL file.
        path: PathBuf,
    },
    /// Extract a random sample from a JSONL file.
    Sample {
        /// Path to JSONL file.
        path: PathBuf,
        /// Number of lines to sample (required, >= 1).
        #[arg(long, short = 'n')]
        count: usize,
        /// Output file (default: stdout).
        #[arg(long, short = 'o')]
        output_file: Option<PathBuf>,
    },
    /// Merge multiple JSONL files into one.
    Merge {
        /// Input JSONL files.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Output file (default: stdout).
        #[arg(long, short = 'o')]
        output_file: Option<PathBuf>,
    },
    /// Split a JSONL file into chunks.
    Split {
        /// Path to JSONL file.
        path: PathBuf,
        /// Maximum lines per chunk.
        #[arg(long, default_value = "1000")]
        chunk_size: usize,
        /// Output directory (default: same as input).
        #[arg(long, short = 'o')]
        output_dir: Option<PathBuf>,
    },
    /// Compare two JSONL result files by custom_id.
    Diff {
        /// First JSONL file.
        a: PathBuf,
        /// Second JSONL file.
        b: PathBuf,
    },
}

impl FileCommands {
    /// Whether this subcommand is a local operation (no API call needed).
    pub fn is_local(&self) -> bool {
        matches!(
            self,
            FileCommands::Validate { .. }
                | FileCommands::Prepare(_)
                | FileCommands::Stats { .. }
                | FileCommands::Sample { .. }
                | FileCommands::Merge { .. }
                | FileCommands::Split { .. }
                | FileCommands::Diff { .. }
        )
    }
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

    /// Write batch ID(s) to a file (one per line).
    #[arg(long)]
    pub output_id: Option<PathBuf>,
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

// --- Keys ---

#[derive(Subcommand)]
pub enum KeyCommands {
    /// Create an API key.
    Create {
        /// Name for the key.
        #[arg(long)]
        name: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// List your API keys (secrets are masked).
    List {
        /// Maximum number of keys to return (default: 20).
        #[arg(long, short = 'n', default_value = "20")]
        limit: u64,
        /// Number of entries to skip (for pagination).
        #[arg(long, default_value = "0")]
        skip: u64,
    },
    /// Delete an API key.
    Delete {
        /// Key ID (UUID).
        id: String,
        /// Skip confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
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

// --- Project ---

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Create a new project with scaffolding and hello-world example.
    Init {
        /// Project name (used for directory and package name).
        name: Option<String>,
        /// Template: single-batch, pipeline, shell, or minimal.
        #[arg(long, short = 't')]
        template: Option<String>,
        /// Add optional SDK dependencies (repeatable: autobatcher, parfold).
        #[arg(long = "with", value_name = "SDK")]
        with_sdks: Vec<String>,
    },
    /// Run project setup (e.g. install dependencies).
    Setup,
    /// Run a named project step from dw.toml.
    Run {
        /// Step name (e.g. "prepare", "analyze").
        step: String,
        /// Extra arguments passed to the step command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run all workflow steps sequentially.
    RunAll {
        /// Start from step N (1-indexed, skips earlier steps).
        #[arg(long, default_value = "1")]
        from: usize,
        /// Continue from the last completed step of a previous run.
        #[arg(long, short = 'c')]
        r#continue: bool,
    },
    /// Show current run status, completed steps, and batch IDs.
    Status,
    /// Clean project artifacts (batches/, results/, run state).
    Clean,
    /// Show available project steps and workflow.
    Info,
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
    #[arg(long, short = 'n', default_value = "20", value_parser = clap::value_parser!(u64).range(1..=100))]
    pub limit: u64,
    /// Number of entries to skip (for pagination).
    #[arg(long, default_value = "0")]
    pub skip: u64,
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
    /// Filter by HTTP status code (100-599).
    #[arg(long, value_parser = clap::value_parser!(u16).range(100..=599))]
    pub status: Option<u16>,
}
