use serde::{Deserialize, Serialize};

/// Response from file upload or file retrieval.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: i64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    #[serde(default)]
    pub created_by_email: Option<String>,
    #[serde(default)]
    pub context_name: Option<String>,
    #[serde(default)]
    pub context_type: Option<String>,
}

/// Parameters for listing files.
#[derive(Debug, Default, Serialize)]
pub struct ListFilesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// File cost estimate response.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileCostEstimate {
    pub total_cost: f64,
    #[serde(default)]
    pub model_breakdowns: Vec<ModelCostBreakdown>,
}

/// Per-model cost breakdown within a cost estimate.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelCostBreakdown {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub request_count: Option<i64>,
    #[serde(default)]
    pub estimated_cost: Option<f64>,
}

/// Response wrapper for file list (OpenAI-compatible cursor pagination).
#[derive(Debug, Deserialize)]
pub struct FileListResponse {
    pub data: Vec<FileResponse>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub first_id: Option<String>,
    #[serde(default)]
    pub last_id: Option<String>,
}
