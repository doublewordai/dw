use crate::config::{self, Config, ServerConfig};

pub fn show(config: &Config) {
    let servers = config.servers.as_ref().cloned().unwrap_or_default();

    println!(
        "Active account: {}",
        config.active_account.as_deref().unwrap_or("(none)")
    );
    println!("Default output: {}", config.default_output);
    println!("Inference API:  {}", servers.ai);
    println!("Admin API:      {}", servers.admin);
}

pub fn set_url(config: &mut Config, url: &str) -> anyhow::Result<()> {
    let url = url.trim_end_matches('/').to_string();
    let servers = config.servers.get_or_insert_with(ServerConfig::default);
    servers.ai = url.clone();
    servers.admin = url;
    config::save_config(config)?;
    eprintln!("Set both server URLs.");
    show(config);
    Ok(())
}

pub fn set_ai_url(config: &mut Config, url: &str) -> anyhow::Result<()> {
    let url = url.trim_end_matches('/').to_string();
    let servers = config.servers.get_or_insert_with(ServerConfig::default);
    servers.ai = url;
    config::save_config(config)?;
    eprintln!("Set inference API URL.");
    show(config);
    Ok(())
}

pub fn set_admin_url(config: &mut Config, url: &str) -> anyhow::Result<()> {
    let url = url.trim_end_matches('/').to_string();
    let servers = config.servers.get_or_insert_with(ServerConfig::default);
    servers.admin = url;
    config::save_config(config)?;
    eprintln!("Set admin API URL.");
    show(config);
    Ok(())
}

pub fn reset_urls(config: &mut Config) -> anyhow::Result<()> {
    config.servers = Some(ServerConfig::default());
    config::save_config(config)?;
    eprintln!("Reset server URLs to defaults.");
    show(config);
    Ok(())
}
