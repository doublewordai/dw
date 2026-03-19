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
    // Validate the key by listing files (lightweight, uses inference API surface)
    let client = dw_client::DwClient::with_inference_key(api_key.to_string())?;

    let params = dw_client::types::files::ListFilesParams {
        limit: Some(1),
        ..Default::default()
    };
    match client.list_files(&params).await {
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

    // Extract the one-time code (no secrets in the redirect URL)
    let code = params
        .get("code")
        .ok_or_else(|| anyhow::anyhow!("No authorization code in callback"))?;

    send_response(
        &stream,
        "Authentication successful! You can close this tab.",
    )
    .await?;

    // Exchange the code for API keys via POST (secrets stay in response body, not URLs)
    eprintln!("Exchanging authorization code...");

    let exchange_url = format!("{}/authentication/cli-exchange", admin_base);
    let http_client = reqwest::Client::new();
    let exchange_response = http_client
        .post(&exchange_url)
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to exchange code: {}", e))?;

    if !exchange_response.status().is_success() {
        let status = exchange_response.status();
        let body = exchange_response.text().await.unwrap_or_default();
        anyhow::bail!("Code exchange failed ({}): {}", status, body);
    }

    let exchange_data: serde_json::Value = exchange_response.json().await?;

    let account_name = exchange_data["account_name"]
        .as_str()
        .unwrap_or("personal")
        .to_string();

    let account = Account {
        display_name: exchange_data["display_name"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        user_id: exchange_data["user_id"].as_str().unwrap_or("").to_string(),
        email: exchange_data["email"].as_str().unwrap_or("").to_string(),
        inference_key: exchange_data["inference_key"]
            .as_str()
            .map(|s| s.to_string()),
        inference_key_id: exchange_data["inference_key_id"]
            .as_str()
            .map(|s| s.to_string()),
        platform_key: exchange_data["platform_key"]
            .as_str()
            .map(|s| s.to_string()),
        platform_key_id: exchange_data["platform_key_id"]
            .as_str()
            .map(|s| s.to_string()),
        org_id: exchange_data["org_id"].as_str().map(|s| s.to_string()),
    };

    credentials.accounts.insert(account_name.clone(), account);
    config.active_account = Some(account_name.clone());

    config::save_credentials(credentials)?;
    config::save_config(config)?;

    eprintln!(
        "Logged in as {}. Active account: {}",
        credentials.accounts[&account_name].display_name, account_name
    );

    Ok(())
}

/// Handle `dw logout`.
///
/// Removes local credentials only. Does NOT delete keys server-side — keys
/// may have been created externally (dashboard, API) and should not be
/// destroyed by the CLI. Users can revoke keys from the dashboard if needed.
pub async fn logout(
    args: &LogoutArgs,
    config: &mut Config,
    credentials: &mut Credentials,
) -> anyhow::Result<()> {
    if args.all {
        credentials.accounts.clear();
        config.active_account = None;
        config::save_credentials(credentials)?;
        config::save_config(config)?;
        eprintln!("Logged out of all accounts. Local credentials removed.");
        return Ok(());
    }

    let account_name = args
        .account
        .as_deref()
        .or(config.active_account.as_deref())
        .ok_or_else(|| anyhow::anyhow!("No active account to log out of."))?
        .to_string();

    if credentials.accounts.remove(&account_name).is_none() {
        anyhow::bail!("Account '{}' not found.", account_name);
    }

    // Update active account
    if config.active_account.as_deref() == Some(&account_name) {
        config.active_account = credentials.accounts.keys().next().cloned();
    }

    config::save_credentials(credentials)?;
    config::save_config(config)?;
    eprintln!(
        "Logged out of '{}'. Local credentials removed.",
        account_name
    );
    eprintln!("API keys are still active — revoke them from the dashboard if needed.");

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
