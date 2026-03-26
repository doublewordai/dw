mod cli;
mod commands;
mod config;
mod jsonl;
mod output;

use clap::Parser;
use cli::{
    AccountCommands, BatchCommands, Commands, ConfigCommands, ExampleCommands, FileCommands,
    KeyCommands, ModelCommands, ProjectCommands, WebhookCommands,
};
use config::{ServerOverrides, build_client, load_config, load_credentials, resolve_account};
use output::OutputFormat;
use std::io::IsTerminal;

#[tokio::main]
async fn main() {
    // Exit cleanly on broken pipe (e.g. `dw files list | head -1`)
    reset_sigpipe();

    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Reset SIGPIPE to default behaviour so broken pipes cause a clean exit
/// instead of a panic. Rust ignores SIGPIPE by default.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

/// Hint on first run when no accounts are stored.
fn maybe_show_welcome(credentials: &config::Credentials) {
    if !credentials.accounts.is_empty() {
        return;
    }
    if !std::io::stderr().is_terminal() {
        return;
    }
    eprintln!(
        "Not logged in. Run `dw login` to authenticate or `dw login --api-key <KEY>` for headless setup."
    );
}

async fn run() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();

    let mut config = load_config();
    let mut credentials = load_credentials();

    let format = cli.output.unwrap_or_else(OutputFormat::default_for_stdout);

    let server_overrides = ServerOverrides {
        both: cli.server.as_deref(),
        ai: cli.server_ai.as_deref(),
        admin: cli.server_admin.as_deref(),
    };

    match cli.command {
        // --- Auth commands (don't require existing credentials) ---
        Commands::Login(args) => commands::auth::login(&args, &mut config, &mut credentials).await,
        Commands::Logout(args) => {
            commands::auth::logout(&args, &mut config, &mut credentials).await
        }
        Commands::Completions(args) => {
            use clap::CommandFactory;
            let mut cmd = cli::Cli::command();
            clap_complete::generate(args.shell, &mut cmd, "dw", &mut std::io::stdout());
            Ok(())
        }
        Commands::Update => commands::update::run().await,
        Commands::Project(subcmd) => match subcmd {
            ProjectCommands::Init {
                name,
                template,
                with_sdks,
            } => commands::project::init(name.as_deref(), template.as_deref(), &with_sdks),
            ProjectCommands::Setup => commands::project::setup(),
            ProjectCommands::Run { step, args } => commands::project::run(&step, &args),
            ProjectCommands::RunAll { from, r#continue } => {
                commands::project::run_all(from, r#continue)
            }
            ProjectCommands::Status => commands::project::status(),
            ProjectCommands::Clean => commands::project::clean(),
            ProjectCommands::Info => commands::project::info(),
        },
        Commands::Examples(subcmd) => match subcmd {
            ExampleCommands::List => {
                commands::examples::list();
                Ok(())
            }
            ExampleCommands::Clone { name, dir } => {
                commands::examples::clone_example(&name, dir.as_deref()).await
            }
        },

        // --- Config commands (local operations) ---
        Commands::Config(subcmd) => match subcmd {
            ConfigCommands::Show => {
                commands::config::show(&config);
                Ok(())
            }
            ConfigCommands::SetUrl { url } => commands::config::set_url(&mut config, &url),
            ConfigCommands::SetAiUrl { url } => commands::config::set_ai_url(&mut config, &url),
            ConfigCommands::SetAdminUrl { url } => {
                commands::config::set_admin_url(&mut config, &url)
            }
            ConfigCommands::ResetUrls => commands::config::reset_urls(&mut config),
        },

        // --- Local file commands (no auth needed) ---
        Commands::Files(ref subcmd) if subcmd.is_local() => match subcmd {
            FileCommands::Validate { path } => commands::files::validate(path),
            FileCommands::Prepare(args) => commands::files::prepare(args).await,
            FileCommands::Stats { path } => commands::files::stats(path, format),
            FileCommands::Sample {
                path,
                count,
                output_file,
            } => commands::files::sample(path, *count, output_file.as_deref()),
            FileCommands::Merge { paths, output_file } => {
                commands::files::merge(paths, output_file.as_deref())
            }
            FileCommands::Split {
                path,
                chunk_size,
                output_dir,
            } => commands::files::split(path, *chunk_size, output_dir.as_deref()),
            FileCommands::Diff { a, b } => commands::files::diff(a, b, format),
            _ => unreachable!(),
        },

        // --- Account commands (local operations) ---
        Commands::Account(subcmd) => match subcmd {
            AccountCommands::List => {
                commands::account::list(&config, &credentials, format);
                Ok(())
            }
            AccountCommands::Switch { name } => {
                commands::account::switch(&name, &mut config, &credentials)
            }
            AccountCommands::Current => {
                commands::account::current(&config, &credentials);
                Ok(())
            }
            AccountCommands::Rename { current, new } => {
                commands::account::rename(&current, &new, &mut config, &mut credentials)
            }
            AccountCommands::Remove { name } => {
                commands::account::remove(&name, &mut config, &mut credentials)
            }
        },

        // --- Commands requiring authentication ---
        cmd => {
            maybe_show_welcome(&credentials);
            let (_account_name, account) =
                resolve_account(cli.account.as_deref(), &config, &credentials)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
            let account = account.clone();
            let client = build_client(&account, &config, &server_overrides)?;
            let client_cfg = config.client.clone().unwrap_or_default();
            let poll_interval = client_cfg.effective_poll_interval();
            let max_retries = client_cfg.max_retries;

            match cmd {
                Commands::Whoami => commands::auth::whoami(&client).await,

                Commands::Models(subcmd) => match subcmd {
                    ModelCommands::List { r#type } => {
                        commands::models::list(&client, r#type.as_deref(), format).await
                    }
                    ModelCommands::Get { model } => {
                        commands::models::get(&client, &model, format).await
                    }
                },

                Commands::Files(subcmd) => match subcmd {
                    FileCommands::Upload(args) => {
                        commands::files::upload(&client, &args, format).await
                    }
                    FileCommands::List {
                        limit,
                        after,
                        all,
                        purpose,
                    } => {
                        commands::files::list(
                            &client,
                            limit,
                            after.as_deref(),
                            all,
                            &purpose,
                            format,
                        )
                        .await
                    }
                    FileCommands::Get { id } => commands::files::get(&client, &id, format).await,
                    FileCommands::Delete { id, yes } => {
                        commands::files::delete(&client, &id, yes).await
                    }
                    FileCommands::Content { id, output_file } => {
                        commands::files::content(&client, &id, output_file.as_deref()).await
                    }
                    FileCommands::CostEstimate {
                        id,
                        completion_window,
                    } => {
                        commands::files::cost_estimate(
                            &client,
                            &id,
                            completion_window.as_deref(),
                            format,
                        )
                        .await
                    }
                    // Local file commands handled in pre-auth branch via is_local()
                    _ => unreachable!("local file commands handled before auth"),
                },

                Commands::Batches(subcmd) => match subcmd {
                    BatchCommands::Create(args) => {
                        commands::batches::create(&client, &args, format).await
                    }
                    BatchCommands::List {
                        limit,
                        after,
                        all,
                        active_first,
                    } => {
                        commands::batches::list(
                            &client,
                            limit,
                            after.as_deref(),
                            all,
                            active_first,
                            format,
                        )
                        .await
                    }
                    BatchCommands::Get { id } => commands::batches::get(&client, &id, format).await,
                    BatchCommands::Cancel { id, yes } => {
                        commands::batches::cancel(&client, &id, yes).await
                    }
                    BatchCommands::Retry { id } => {
                        commands::batches::retry(&client, &id, format).await
                    }
                    BatchCommands::Results { id, output_file } => {
                        commands::batches::results(&client, &id, output_file.as_deref()).await
                    }
                    BatchCommands::Run(args) => {
                        commands::batches::run(&client, &args, format, poll_interval, max_retries)
                            .await
                    }
                    BatchCommands::Watch { ids } => {
                        commands::batches::watch_batches(&client, &ids, poll_interval, max_retries)
                            .await
                    }
                    BatchCommands::Analytics { id } => {
                        commands::usage::batch_analytics(&client, &id, format).await
                    }
                },

                Commands::Stream(args) => {
                    commands::stream::run(&client, &args, poll_interval, max_retries).await
                }

                Commands::Realtime(args) => commands::realtime::run(&client, &args).await,

                Commands::Usage(args) => commands::usage::run(&client, &args, format).await,

                Commands::Requests(args) => {
                    commands::usage::list_requests(&client, &args, format).await
                }

                Commands::Keys(subcmd) => match subcmd {
                    KeyCommands::Create { name, description } => {
                        commands::keys::create(&client, &name, description.as_deref(), format).await
                    }
                    KeyCommands::List { limit, skip } => {
                        commands::keys::list(&client, limit, skip, format).await
                    }
                    KeyCommands::Delete { id, yes } => {
                        commands::keys::delete(&client, &id, yes).await
                    }
                },

                Commands::Webhooks(subcmd) => match subcmd {
                    WebhookCommands::Create {
                        url,
                        events,
                        description,
                    } => {
                        commands::webhooks::create(
                            &client,
                            &account,
                            &url,
                            events.as_deref(),
                            description.as_deref(),
                            format,
                        )
                        .await
                    }
                    WebhookCommands::List => {
                        commands::webhooks::list(&client, &account, format).await
                    }
                    WebhookCommands::Delete { id, yes } => {
                        commands::webhooks::delete(&client, &account, &id, yes).await
                    }
                    WebhookCommands::RotateSecret { id } => {
                        commands::webhooks::rotate_secret(&client, &account, &id).await
                    }
                },

                // These are handled above and won't reach here
                Commands::Login(_)
                | Commands::Logout(_)
                | Commands::Account(_)
                | Commands::Config(_)
                | Commands::Examples(_)
                | Commands::Completions(_)
                | Commands::Update
                | Commands::Project(_) => unreachable!(),
            }
        }
    }
}
