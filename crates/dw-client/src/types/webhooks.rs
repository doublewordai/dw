use serde::{Deserialize, Serialize};

/// Request to create a webhook.
#[derive(Debug, Serialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request to update a webhook.
#[derive(Debug, Serialize)]
pub struct UpdateWebhookRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Webhook response.
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub id: String,
    pub user_id: String,
    pub url: String,
    pub enabled: bool,
    #[serde(default)]
    pub event_types: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Webhook response with secret (returned on create and rotate).
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookWithSecretResponse {
    #[serde(flatten)]
    pub webhook: WebhookResponse,
    pub secret: String,
}
