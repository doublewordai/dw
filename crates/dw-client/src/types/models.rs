use serde::{Deserialize, Serialize};

/// Deployed model response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: String,
    pub object: String,
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
    pub created: i64,
    pub owned_by: String,
}

/// Model list response (OpenAI-compatible format).
#[derive(Debug, Deserialize)]
pub struct ModelListResponse {
    pub data: Vec<ModelResponse>,
    pub object: String,
}
