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
        let display = account.effective_display(name);
        println!(
            " {} {} (type: {}, email: {})",
            marker, display, account.account_type, account.email
        );
    }

    if credentials.accounts.len() > 1 {
        let names: Vec<_> = credentials
            .accounts
            .iter()
            .map(|(name, a)| a.effective_display(name))
            .collect();
        eprintln!("\nSwitch with: dw account switch <name>");
        eprintln!("Available: {}", names.join(", "));
    }
}

pub fn switch(name: &str, config: &mut Config, credentials: &Credentials) -> anyhow::Result<()> {
    // Try exact key match first
    let by_key = credentials
        .accounts
        .keys()
        .find(|k| k.eq_ignore_ascii_case(name))
        .cloned();

    let found = if let Some(key) = by_key {
        Some(key)
    } else {
        // Try display name match, detect ambiguity
        let matches: Vec<_> = credentials
            .accounts
            .iter()
            .filter(|(k, a)| a.effective_display(k).eq_ignore_ascii_case(name))
            .map(|(k, _)| k.clone())
            .collect();

        if matches.len() == 1 {
            Some(matches.into_iter().next().unwrap())
        } else if matches.len() > 1 {
            anyhow::bail!(
                "Multiple accounts match '{}'. Switch by key instead:\n  {}",
                name,
                matches
                    .iter()
                    .map(|k| format!("dw account switch {}", k))
                    .collect::<Vec<_>>()
                    .join("\n  ")
            );
        } else {
            None
        }
    };

    match found {
        Some(key) => {
            let display = credentials.accounts[&key]
                .effective_display(&key)
                .to_string();
            config.active_account = Some(key);
            config::save_config(config)?;
            eprintln!("Switched to account: {}", display);
            Ok(())
        }
        None => {
            let available: Vec<_> = credentials
                .accounts
                .iter()
                .map(|(name, a)| a.effective_display(name))
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
                let display = account.effective_display(name);
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
