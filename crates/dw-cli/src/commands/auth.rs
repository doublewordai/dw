use crate::cli::{LoginArgs, LogoutArgs};
use crate::config::{self, Account, Config, Credentials};

/// Handle `dw login`.
pub async fn login(
    args: &LoginArgs,
    config: &mut Config,
    credentials: &mut Credentials,
) -> anyhow::Result<()> {
    if let Some(ref api_key) = args.api_key {
        // Headless login: store API key directly
        login_with_key(api_key, credentials, config).await
    } else {
        // Browser login flow
        login_browser(args.org.as_deref(), credentials, config).await
    }
}

/// Login with a provided API key (headless/agent mode).
async fn login_with_key(
    api_key: &str,
    credentials: &mut Credentials,
    config: &mut Config,
) -> anyhow::Result<()> {
    // Try to validate the key by making a lightweight request
    let client = dw_client::DwClient::with_inference_key(api_key.to_string())?;

    // Try listing models as a lightweight auth check
    match client.list_models().await {
        Ok(_) => {}
        Err(dw_client::DwError::Unauthenticated) => {
            anyhow::bail!("Invalid API key. Check the key and try again.");
        }
        Err(e) => {
            eprintln!("Warning: could not verify key ({}). Storing anyway.", e);
        }
    }

    let account_name = "default".to_string();
    let account = Account {
        display_name: "API Key".to_string(),
        user_id: "unknown".to_string(),
        email: "unknown".to_string(),
        inference_key: Some(api_key.to_string()),
        inference_key_id: None,
        platform_key: None,
        platform_key_id: None,
        org_id: None,
    };

    credentials.accounts.insert(account_name.clone(), account);
    config.active_account = Some(account_name);

    config::save_credentials(credentials)?;
    config::save_config(config)?;

    eprintln!("Logged in with API key. Active account: default");
    eprintln!(
        "Note: some commands (webhooks, whoami) require full login via `dw login` (browser flow)."
    );
    Ok(())
}

/// Browser-based login flow.
async fn login_browser(
    org: Option<&str>,
    credentials: &mut Credentials,
    config: &mut Config,
) -> anyhow::Result<()> {
    use tokio::net::TcpListener;

    // Generate state for CSRF protection
    let state: String = {
        use rand::Rng;
        let mut rng = rand::rng();
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect()
    };

    // Bind to random available port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    // Build auth URL
    let admin_base = config
        .servers
        .as_ref()
        .map(|s| s.admin.as_str())
        .unwrap_or("https://app.doubleword.ai");

    let mut callback_url = format!(
        "{}/authentication/cli-callback?port={}&state={}",
        admin_base, port, state
    );
    if let Some(org_slug) = org {
        callback_url.push_str(&format!("&org={}", org_slug));
    }

    let auth_url = format!(
        "{}/authentication/sign_in?rd={}",
        admin_base,
        urlencoding::encode(&callback_url)
    );

    eprintln!("Opening browser for authentication...");
    eprintln!("If the browser doesn't open, visit: {}", auth_url);

    if open::that(&auth_url).is_err() {
        eprintln!("Could not open browser. Please visit the URL above manually.");
    }

    // Wait for callback
    eprintln!("Waiting for authentication (press Ctrl+C to cancel)...");

    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(300), listener.accept())
        .await
        .map_err(|_| anyhow::anyhow!("Login timed out after 5 minutes."))??;

    // Read the HTTP request
    let mut buf = vec![0u8; 4096];
    stream.readable().await?;
    let n = stream.try_read(&mut buf)?;
    let request_str = String::from_utf8_lossy(&buf[..n]);

    // Parse query params from the request path
    let path = request_str
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let params = parse_query_params(path);

    // Validate state
    let returned_state = params.get("state").map(|s| s.as_str()).unwrap_or("");
    if returned_state != state {
        send_response(&stream, "Authentication failed: invalid state parameter.").await?;
        anyhow::bail!("CSRF state mismatch. Login aborted.");
    }

    // Extract keys and user info
    let inference_key = params
        .get("inference_key")
        .ok_or_else(|| anyhow::anyhow!("No realtime key in callback"))?;
    let platform_key = params
        .get("platform_key")
        .ok_or_else(|| anyhow::anyhow!("No platform key in callback"))?;

    let user_id = params.get("user_id").cloned().unwrap_or_default();
    let email = params.get("email").cloned().unwrap_or_default();
    let display_name = params
        .get("display_name")
        .cloned()
        .unwrap_or_else(|| email.clone());
    let account_name = params
        .get("account_name")
        .cloned()
        .unwrap_or_else(|| "personal".to_string());
    let org_id = params.get("org_id").cloned();

    // Store account
    let account = Account {
        display_name,
        user_id,
        email,
        inference_key: Some(inference_key.clone()),
        inference_key_id: params.get("inference_key_id").cloned(),
        platform_key: Some(platform_key.clone()),
        platform_key_id: params.get("platform_key_id").cloned(),
        org_id,
    };

    credentials.accounts.insert(account_name.clone(), account);
    config.active_account = Some(account_name.clone());

    config::save_credentials(credentials)?;
    config::save_config(config)?;

    send_response(
        &stream,
        "Authentication successful! You can close this tab.",
    )
    .await?;

    eprintln!(
        "Logged in as {}. Active account: {}",
        credentials.accounts[&account_name].display_name, account_name
    );

    Ok(())
}

/// Handle `dw logout`.
pub async fn logout(
    args: &LogoutArgs,
    config: &mut Config,
    credentials: &mut Credentials,
) -> anyhow::Result<()> {
    if args.all {
        // Attempt to delete keys server-side for each account (best effort)
        for (name, account) in &credentials.accounts {
            if let Err(e) = delete_account_keys(account, config).await {
                eprintln!("Warning: could not revoke keys for '{}': {}", name, e);
            }
        }
        credentials.accounts.clear();
        config.active_account = None;
        config::save_credentials(credentials)?;
        config::save_config(config)?;
        eprintln!("Logged out of all accounts.");
        return Ok(());
    }

    let account_name = args
        .account
        .as_deref()
        .or(config.active_account.as_deref())
        .ok_or_else(|| anyhow::anyhow!("No active account to log out of."))?
        .to_string();

    if let Some(account) = credentials.accounts.get(&account_name)
        && let Err(e) = delete_account_keys(account, config).await
    {
        eprintln!("Warning: could not revoke keys: {}", e);
    }

    credentials.accounts.remove(&account_name);

    // Update active account
    if config.active_account.as_deref() == Some(&account_name) {
        config.active_account = credentials.accounts.keys().next().cloned();
    }

    config::save_credentials(credentials)?;
    config::save_config(config)?;
    eprintln!("Logged out of '{}'.", account_name);

    Ok(())
}

/// Handle `dw whoami`.
pub async fn whoami(client: &dw_client::DwClient) -> anyhow::Result<()> {
    let user = client.get_current_user().await?;
    println!(
        "User:    {} ({})",
        user.display_name.unwrap_or(user.username),
        user.email
    );
    println!("ID:      {}", user.id);
    if let Some(roles) = &user.roles {
        println!("Roles:   {}", roles.join(", "));
    }
    if let Some(balance) = user.credit_balance {
        println!("Credits: ${:.2}", balance);
    }
    if let Some(orgs) = &user.organizations
        && !orgs.is_empty()
    {
        println!("Orgs:");
        for org in orgs {
            let name = org
                .display_name
                .as_deref()
                .or(org.name.as_deref())
                .unwrap_or(&org.id);
            let role = org.role.as_deref().unwrap_or("member");
            println!("  - {} ({})", name, role);
        }
    }
    Ok(())
}

/// Best-effort key deletion on logout.
async fn delete_account_keys(account: &Account, config: &Config) -> anyhow::Result<()> {
    let no_overrides = config::ServerOverrides {
        both: None,
        ai: None,
        admin: None,
    };
    let client = config::build_client(account, config, &no_overrides)?;

    // Delete inference key first (using platform key)
    if let (Some(key_id), Some(_)) = (&account.inference_key_id, &account.platform_key) {
        client.delete_api_key(&account.user_id, key_id).await.ok();
    }

    // Delete platform key (self-delete)
    if let (Some(key_id), Some(_)) = (&account.platform_key_id, &account.platform_key) {
        client.delete_api_key(&account.user_id, key_id).await.ok();
    }

    Ok(())
}

fn parse_query_params(path: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    if let Some(query) = path.split('?').nth(1) {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding::decode(key).unwrap_or_default().into_owned(),
                    urlencoding::decode(value).unwrap_or_default().into_owned(),
                );
            }
        }
    }
    params
}

async fn send_response(stream: &tokio::net::TcpStream, message: &str) -> anyhow::Result<()> {
    let html = format!(
        r#"<!DOCTYPE html>
<html><head><title>DW CLI</title></head>
<body style="font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0;">
<div style="text-align: center;">
<h2>{}</h2>
<p style="color: #666;">You can close this tab and return to your terminal.</p>
</div>
</body></html>"#,
        message
    );

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );

    stream.writable().await?;
    stream.try_write(response.as_bytes())?;
    Ok(())
}
