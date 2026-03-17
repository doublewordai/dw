use crate::config::{self, Config, Credentials};
use crate::output::OutputFormat;

pub fn list(config: &Config, credentials: &Credentials, _format: OutputFormat) {
    if credentials.accounts.is_empty() {
        eprintln!("No accounts stored. Run `dw login` to authenticate.");
        return;
    }

    let active = config.active_account.as_deref().unwrap_or("");

    for (name, account) in &credentials.accounts {
        let marker = if name == active { " *" } else { "" };
        let context = account
            .org_id
            .as_ref()
            .map(|_| " (org)")
            .unwrap_or(" (personal)");
        println!("  {}{}{} — {}", name, context, marker, account.email);
    }
}

pub fn switch(name: &str, config: &mut Config, credentials: &Credentials) -> anyhow::Result<()> {
    // Find account (case-insensitive)
    let found = credentials
        .accounts
        .keys()
        .find(|k| k.eq_ignore_ascii_case(name));

    match found {
        Some(key) => {
            config.active_account = Some(key.clone());
            config::save_config(config)?;
            eprintln!("Switched to account: {}", key);
            Ok(())
        }
        None => {
            let available: Vec<_> = credentials.accounts.keys().collect();
            anyhow::bail!(
                "Account '{}' not found. Available: {}",
                name,
                available
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

pub fn current(config: &Config, credentials: &Credentials) {
    match config.active_account.as_deref() {
        Some(name) => {
            if let Some(account) = credentials.accounts.get(name) {
                let context = if account.org_id.is_some() {
                    "org"
                } else {
                    "personal"
                };
                println!("{} ({}) — {}", name, context, account.email);
            } else {
                println!("{} (not found in credentials)", name);
            }
        }
        None => {
            eprintln!("No active account. Run `dw login` to authenticate.");
        }
    }
}
