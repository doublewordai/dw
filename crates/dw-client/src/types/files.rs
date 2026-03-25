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
    pub file_id: String,
    pub total_requests: i64,
    pub total_estimated_input_tokens: i64,
    pub total_estimated_output_tokens: i64,
    /// Cost as string to preserve decimal precision.
    pub total_estimated_cost: String,
    pub models: Vec<ModelCostBreakdown>,
}

/// Per-model cost breakdown within a cost estimate.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelCostBreakdown {
    pub model: String,
    pub request_count: i64,
    pub estimated_input_tokens: i64,
    pub estimated_output_tokens: i64,
    /// Cost as string to preserve decimal precision.
    pub estimated_cost: String,
}

/// Response wrapper for file list (OpenAI-compatible cursor pagination).
/// Result from streaming file content with offset.
#[derive(Debug)]
pub enum FileContentChunk {
    /// New content available.
    Data {
        /// The JSONL content (may be multiple lines).
        body: String,
        /// Offset to use for the next request (from X-Last-Line header).
        next_offset: usize,
        /// Whether more content may follow (from X-Incomplete header).
        incomplete: bool,
    },
    /// File does not exist yet (404). The output file is created when the
    /// first results arrive — this is normal during early polling.
    NotReady,
}

#[derive(Debug, Deserialize)]
pub struct FileListResponse {
    #[serde(default)]
    pub object: String,
    pub data: Vec<FileResponse>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub first_id: Option<String>,
    #[serde(default)]
    pub last_id: Option<String>,
}
