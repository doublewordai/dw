use serde::{Deserialize, Serialize};

/// Deployed model response from the admin API.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: String,
    pub model_name: String,
    pub alias: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    #[serde(default)]
    pub capabilities: Option<Vec<String>>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub hosted_on: Option<String>,
    #[serde(default)]
    pub requests_per_second: Option<f32>,
    #[serde(default)]
    pub burst_size: Option<i32>,
    #[serde(default)]
    pub capacity: Option<i32>,
    #[serde(default)]
    pub batch_capacity: Option<i32>,
}

/// Model list response from the admin API.
#[derive(Debug, Deserialize)]
pub struct ModelListResponse {
    pub data: Vec<ModelResponse>,
    pub total_count: i64,
    pub skip: i64,
    pub limit: i64,
}
