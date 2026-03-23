use crate::config::{self, Config, Credentials};
use crate::output::OutputFormat;

/// Resolve an account name to its key, handling ambiguity.
/// Tries exact key match first, then display name. Errors if multiple display name matches.
fn resolve_name(name: &str, credentials: &Credentials) -> anyhow::Result<String> {
    // Exact key match (case-insensitive)
    if let Some(key) = credentials
        .accounts
        .keys()
        .find(|k| k.eq_ignore_ascii_case(name))
    {
        return Ok(key.clone());
    }

    // Display name match — detect ambiguity
    let matches: Vec<_> = credentials
        .accounts
        .iter()
        .filter(|(k, a)| a.effective_display(k).eq_ignore_ascii_case(name))
        .map(|(k, _)| k.clone())
        .collect();

    match matches.len() {
        1 => Ok(matches.into_iter().next().unwrap()),
        n if n > 1 => anyhow::bail!(
            "Multiple accounts match '{}'. Use the exact key:\n  {}",
            name,
            matches
                .iter()
                .map(|k| format!("dw account switch {}", k))
                .collect::<Vec<_>>()
                .join("\n  ")
        ),
        _ => anyhow::bail!(
            "Account '{}' not found. Run `dw account list` to see available accounts.",
            name
        ),
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
        println!(
            " {} {} (type: {}, email: {})",
            marker, name, account.account_type, account.email
        );
    }

    if credentials.accounts.len() > 1 {
        let names: Vec<_> = credentials.accounts.keys().map(|k| k.as_str()).collect();
        eprintln!("\nSwitch with: dw account switch <name>");
        eprintln!("Available: {}", names.join(", "));
    }
}

pub fn switch(name: &str, config: &mut Config, credentials: &Credentials) -> anyhow::Result<()> {
    let key = resolve_name(name, credentials)?;
    config.active_account = Some(key.clone());
    config::save_config(config)?;
    eprintln!("Switched to account: {}", key);
    Ok(())
}

pub fn rename(
    current: &str,
    new: &str,
    config: &mut Config,
    credentials: &mut Credentials,
) -> anyhow::Result<()> {
    let key = resolve_name(current, credentials)?;

    // Case-insensitive uniqueness check for the new name
    if credentials
        .accounts
        .keys()
        .any(|k| k.eq_ignore_ascii_case(new))
    {
        anyhow::bail!("Account '{}' already exists.", new);
    }

    let account = credentials.accounts.remove(&key).unwrap();
    credentials.accounts.insert(new.to_string(), account);

    if config.active_account.as_deref() == Some(&key) {
        config.active_account = Some(new.to_string());
    }

    config::save_credentials(credentials)?;
    config::save_config(config)?;
    eprintln!("Renamed '{}' → '{}'", key, new);
    Ok(())
}

pub fn remove(
    name: &str,
    config: &mut Config,
    credentials: &mut Credentials,
) -> anyhow::Result<()> {
    let key = resolve_name(name, credentials)?;

    credentials.accounts.remove(&key);

    if config.active_account.as_deref() == Some(key.as_str()) {
        config.active_account = credentials.accounts.keys().next().cloned();
    }

    config::save_credentials(credentials)?;
    config::save_config(config)?;
    eprintln!("Removed account: {}", key);
    Ok(())
}

pub fn current(config: &Config, credentials: &Credentials) {
    match config.active_account.as_deref() {
        Some(name) => {
            if let Some(account) = credentials.accounts.get(name) {
                println!(
                    "{} (type: {}, email: {})",
                    name, account.account_type, account.email
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
