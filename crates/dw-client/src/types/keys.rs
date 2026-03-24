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

/// API key response (returned on creation — includes the secret).
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// The actual key secret. Only shown on creation.
    pub key: String,
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

/// API key info response (returned on list/get — no secret).
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyInfoResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
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

/// Paginated response for API key lists.
#[derive(Debug, Deserialize)]
pub struct PaginatedApiKeys {
    pub data: Vec<ApiKeyInfoResponse>,
    pub total_count: i64,
    pub skip: i64,
    pub limit: i64,
}
