use crate::output::OutputFormat;
use dw_client::DwClient;
use dw_client::types::keys::CreateApiKeyRequest;

/// Create an API key.
pub async fn create(
    client: &DwClient,
    name: &str,
    description: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let request = CreateApiKeyRequest {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        purpose: None, // defaults to "realtime"
    };

    let key = client.create_api_key(&request).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&key)?);
        }
        OutputFormat::Plain => {
            println!("{}", key.key);
        }
        OutputFormat::Table => {
            println!("Created API key: {}", key.name);
            println!("  ID:      {}", key.id);
            println!("  Key:     {}", key.key);
            println!(
                "  Purpose: {}",
                key.purpose.as_deref().unwrap_or("realtime")
            );
            println!();
            eprintln!("Save this key — it won't be shown again.");
        }
    }

    Ok(())
}

/// List API keys.
pub async fn list(client: &DwClient, format: OutputFormat) -> anyhow::Result<()> {
    let response = client.list_api_keys().await?;

    // Filter out hidden CLI keys (DW CLI prefix) to avoid confusion
    let keys: Vec<_> = response
        .data
        .iter()
        .filter(|k| !k.name.starts_with("DW CLI"))
        .collect();

    match format {
        OutputFormat::Json => {
            for key in &keys {
                println!("{}", serde_json::to_string(key)?);
            }
        }
        OutputFormat::Plain => {
            for key in &keys {
                println!(
                    "{}\t{}\t{}",
                    key.id,
                    key.name,
                    key.purpose.as_deref().unwrap_or("-")
                );
            }
        }
        OutputFormat::Table => {
            if keys.is_empty() {
                eprintln!("No API keys found.");
                return Ok(());
            }

            let mut table = comfy_table::Table::new();
            table.set_header(vec!["ID", "Name", "Purpose", "Created", "Last Used"]);

            for key in &keys {
                table.add_row(vec![
                    key.id.clone(),
                    key.name.clone(),
                    key.purpose.as_deref().unwrap_or("-").to_string(),
                    key.created_at
                        .as_deref()
                        .map(truncate_timestamp)
                        .unwrap_or_else(|| "-".to_string()),
                    key.last_used
                        .as_deref()
                        .map(truncate_timestamp)
                        .unwrap_or_else(|| "never".to_string()),
                ]);
            }

            println!("{}", table);
        }
    }

    Ok(())
}

/// Delete an API key.
pub async fn delete(client: &DwClient, key_id: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        eprint!("Delete API key {}? [y/N] ", key_id);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }

    client.delete_api_key(key_id).await?;
    eprintln!("Deleted API key: {}", key_id);
    Ok(())
}

fn truncate_timestamp(ts: &str) -> String {
    ts.replace('T', " ")
        .split('.')
        .next()
        .unwrap_or(ts)
        .to_string()
}
