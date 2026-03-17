use serde::{Deserialize, Serialize};

/// Request to create an API key.
#[derive(Debug, Serialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// API key response.
#[derive(Debug, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    /// The actual key secret. Only present on creation.
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub last_used: Option<String>,
}
