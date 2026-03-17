use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Global CLI configuration stored in ~/.dw/config.toml.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub active_account: Option<String>,
    #[serde(default = "default_output")]
    pub default_output: String,
    #[serde(default)]
    pub servers: Option<ServerConfig>,
}

fn default_output() -> String {
    "table".to_string()
}

/// Server URL overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_ai_url")]
    pub ai: String,
    #[serde(default = "default_admin_url")]
    pub admin: String,
}

fn default_ai_url() -> String {
    "https://api.doubleword.ai".to_string()
}

fn default_admin_url() -> String {
    "https://app.doubleword.ai".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ai: default_ai_url(),
            admin: default_admin_url(),
        }
    }
}

/// Credentials stored in ~/.dw/credentials.toml.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(default)]
    pub accounts: BTreeMap<String, Account>,
}

/// A stored account (personal or org-scoped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub display_name: String,
    pub user_id: String,
    pub email: String,
    #[serde(default)]
    pub realtime_key: Option<String>,
    #[serde(default)]
    pub realtime_key_id: Option<String>,
    #[serde(default)]
    pub platform_key: Option<String>,
    #[serde(default)]
    pub platform_key_id: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
}

/// Get the DW config directory path.
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".dw")
}

/// Ensure the config directory exists.
pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Load config from ~/.dw/config.toml.
pub fn load_config() -> Config {
    let path = config_dir().join("config.toml");
    load_toml_file(&path).unwrap_or_default()
}

/// Save config to ~/.dw/config.toml.
pub fn save_config(config: &Config) -> std::io::Result<()> {
    let dir = ensure_config_dir()?;
    let path = dir.join("config.toml");
    let contents = toml::to_string_pretty(config)
        .map_err(|e| std::io::Error::other(format!("TOML serialize error: {e}")))?;
    std::fs::write(&path, contents)
}

/// Load credentials from ~/.dw/credentials.toml.
pub fn load_credentials() -> Credentials {
    let path = config_dir().join("credentials.toml");
    load_toml_file(&path).unwrap_or_default()
}

/// Save credentials to ~/.dw/credentials.toml with restricted permissions.
pub fn save_credentials(creds: &Credentials) -> std::io::Result<()> {
    let dir = ensure_config_dir()?;
    let path = dir.join("credentials.toml");
    let contents = toml::to_string_pretty(creds)
        .map_err(|e| std::io::Error::other(format!("TOML serialize error: {e}")))?;
    std::fs::write(&path, &contents)?;

    // Set file permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Resolve the active account from CLI flag, config, or error.
pub fn resolve_account<'a>(
    account_override: Option<&str>,
    config: &Config,
    credentials: &'a Credentials,
) -> Result<(&'a str, &'a Account), String> {
    let account_name = account_override
        .or(config.active_account.as_deref())
        .ok_or_else(|| {
            "No active account. Run `dw login` to authenticate or `dw account switch <name>` to select an account.".to_string()
        })?;

    // Find in credentials (case-insensitive key lookup)
    let (key, account) = credentials
        .accounts
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(account_name))
        .ok_or_else(|| {
            format!(
                "Account '{}' not found. Run `dw account list` to see available accounts.",
                account_name
            )
        })?;

    Ok((key, account))
}

/// Build a DwClient from resolved account and config.
pub fn build_client(
    account: &Account,
    config: &Config,
    server_override: Option<&str>,
) -> Result<dw_client::DwClient, dw_client::DwError> {
    let servers = config.servers.as_ref().cloned().unwrap_or_default();

    let ai_base_url = server_override.map(|s| s.to_string()).unwrap_or(servers.ai);
    let admin_base_url = servers.admin;

    dw_client::DwClient::new(dw_client::DwClientConfig {
        ai_base_url,
        admin_base_url,
        realtime_key: account.realtime_key.clone(),
        platform_key: account.platform_key.clone(),
        ..Default::default()
    })
}

fn load_toml_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let contents = std::fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}
