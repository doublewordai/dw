use crate::config::{self, Config, Credentials};
use crate::output::OutputFormat;

/// Get effective display name for an account, never empty.
fn effective_display(name: &str, account: &crate::config::Account) -> String {
    let dn = &account.display_name;
    if dn.is_empty() {
        account
            .email
            .split('@')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string())
    } else {
        dn.clone()
    }
}

pub fn list(config: &Config, credentials: &Credentials, _format: OutputFormat) {
    if credentials.accounts.is_empty() {
        eprintln!("No accounts stored. Run `dw login` to authenticate.");
        return;
    }

    let active = config.active_account.as_deref().unwrap_or("");

    for (name, account) in &credentials.accounts {
        let marker = if name == active { "*" } else { " " };
        let display = effective_display(name, account);
        println!(
            " {} {} (type: {}, email: {})",
            marker, display, account.account_type, account.email
        );
    }

    if credentials.accounts.len() > 1 {
        let names: Vec<_> = credentials
            .accounts
            .iter()
            .map(|(name, a)| effective_display(name, a))
            .collect();
        eprintln!("\nSwitch with: dw account switch <name>");
        eprintln!("Available: {}", names.join(", "));
    }
}

pub fn switch(name: &str, config: &mut Config, credentials: &Credentials) -> anyhow::Result<()> {
    // Try exact key match first, then effective display name match
    let found = credentials
        .accounts
        .keys()
        .find(|k| k.eq_ignore_ascii_case(name))
        .cloned()
        .or_else(|| {
            credentials
                .accounts
                .iter()
                .find(|(k, a)| effective_display(k, a).eq_ignore_ascii_case(name))
                .map(|(k, _)| k.clone())
        });

    match found {
        Some(key) => {
            let display = effective_display(&key, &credentials.accounts[&key]);
            config.active_account = Some(key);
            config::save_config(config)?;
            eprintln!("Switched to account: {}", display);
            Ok(())
        }
        None => {
            let available: Vec<_> = credentials
                .accounts
                .iter()
                .map(|(name, a)| effective_display(name, a))
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
                let display = effective_display(name, account);
                println!(
                    "{} (type: {}, email: {})",
                    display, account.account_type, account.email
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
