use dw_client::DwClient;
use dw_client::types::webhooks::{CreateWebhookRequest, WebhookResponse};

use crate::config::Account;
use crate::output::{Displayable, OutputFormat, print_list};

impl Displayable for WebhookResponse {
    fn table_headers() -> Vec<&'static str> {
        vec!["ID", "URL", "Events", "Enabled", "Created"]
    }

    fn to_table_row(&self) -> Vec<String> {
        let events = self
            .event_types
            .as_ref()
            .map(|e| e.join(", "))
            .unwrap_or_else(|| "all".to_string());
        vec![
            self.id.clone(),
            self.url.clone(),
            events,
            self.enabled.to_string(),
            self.created_at.clone(),
        ]
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn to_plain(&self) -> String {
        format!("{}\t{}\t{}", self.id, self.url, self.enabled)
    }
}

pub async fn create(
    client: &DwClient,
    account: &Account,
    url: &str,
    events: Option<&str>,
    description: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let event_types = events.map(|e| e.split(',').map(|s| s.trim().to_string()).collect());

    let request = CreateWebhookRequest {
        url: url.to_string(),
        event_types,
        description: description.map(|d| d.to_string()),
    };

    let response = client.create_webhook(&account.user_id, &request).await?;
    eprintln!("Created webhook: {}", response.webhook.id);
    eprintln!("Signing secret: {}", response.secret);
    eprintln!("Save this secret — it won't be shown again.");

    if format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&response.webhook)?);
    }

    Ok(())
}

pub async fn list(
    client: &DwClient,
    account: &Account,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let webhooks = client.list_webhooks(&account.user_id).await?;
    print_list(&webhooks, format);
    Ok(())
}

pub async fn delete(
    client: &DwClient,
    account: &Account,
    webhook_id: &str,
    yes: bool,
) -> anyhow::Result<()> {
    if !yes {
        eprint!("Delete webhook {}? [y/N] ", webhook_id);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    client.delete_webhook(&account.user_id, webhook_id).await?;
    eprintln!("Deleted webhook {}.", webhook_id);
    Ok(())
}

pub async fn rotate_secret(
    client: &DwClient,
    account: &Account,
    webhook_id: &str,
) -> anyhow::Result<()> {
    let response = client
        .rotate_webhook_secret(&account.user_id, webhook_id)
        .await?;
    eprintln!("Rotated secret for webhook {}.", webhook_id);
    eprintln!("New signing secret: {}", response.secret);
    eprintln!("Save this secret — it won't be shown again.");
    Ok(())
}
