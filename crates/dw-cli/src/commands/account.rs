use crate::config::{self, Config, Credentials};
use crate::output::OutputFormat;

pub fn list(config: &Config, credentials: &Credentials, _format: OutputFormat) {
    if credentials.accounts.is_empty() {
        eprintln!("No accounts stored. Run `dw login` to authenticate.");
        return;
    }

    let active = config.active_account.as_deref().unwrap_or("");

    for (name, account) in &credentials.accounts {
        let marker = if name == active { "*" } else { " " };
        println!(
            " {} {} (type: {}, email: {})",
            marker, account.display_name, account.account_type, account.email
        );
    }

    if credentials.accounts.len() > 1 {
        let names: Vec<_> = credentials
            .accounts
            .values()
            .map(|a| a.display_name.as_str())
            .collect();
        eprintln!("\nSwitch with: dw account switch <name>");
        eprintln!("Available: {}", names.join(", "));
    }
}

pub fn switch(name: &str, config: &mut Config, credentials: &Credentials) -> anyhow::Result<()> {
    // Find account by display name (case-insensitive), fall back to internal key
    let found = credentials
        .accounts
        .iter()
        .find(|(_, a)| a.display_name.eq_ignore_ascii_case(name))
        .map(|(k, _)| k.clone())
        .or_else(|| {
            credentials
                .accounts
                .keys()
                .find(|k| k.eq_ignore_ascii_case(name))
                .cloned()
        });

    match found {
        Some(key) => {
            let display = &credentials.accounts[&key].display_name;
            config.active_account = Some(key);
            config::save_config(config)?;
            eprintln!("Switched to account: {}", display);
            Ok(())
        }
        None => {
            let available: Vec<_> = credentials
                .accounts
                .values()
                .map(|a| a.display_name.as_str())
                .collect();
            anyhow::bail!(
                "Account '{}' not found. Available: {}",
                name,
                available.join(", ")
            );
        }
    }
}

pub fn current(config: &Config, credentials: &Credentials) {
    match config.active_account.as_deref() {
        Some(name) => {
            if let Some(account) = credentials.accounts.get(name) {
                println!(
                    "{} (type: {}, email: {})",
                    account.display_name, account.account_type, account.email
                );
            } else {
                println!("{} (not found in credentials)", name);
            }
        }
        None => {
            eprintln!("No active account. Run `dw login` to authenticate.");
        }
    }
}
