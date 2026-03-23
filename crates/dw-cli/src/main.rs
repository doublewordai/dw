mod cli;
mod commands;
mod config;
mod jsonl;
mod output;

use clap::Parser;
use cli::{
    AccountCommands, BatchCommands, Commands, ConfigCommands, ExampleCommands, FileCommands,
    ModelCommands, WebhookCommands,
};
use config::{ServerOverrides, build_client, load_config, load_credentials, resolve_account};
use output::OutputFormat;

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
                    FileCommands::Validate { path } => commands::files::validate(&path),
                    FileCommands::Prepare(args) => commands::files::prepare(&args).await,
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
                },

                Commands::Stream(args) => {
                    commands::stream::run(&client, &args, poll_interval, max_retries).await
                }

                Commands::Realtime(args) => commands::realtime::run(&client, &args).await,

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
                | Commands::Completions(_) => unreachable!(),
            }
        }
    }
}
