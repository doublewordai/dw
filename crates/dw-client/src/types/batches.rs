use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to create a batch.
#[derive(Debug, Serialize)]
pub struct CreateBatchRequest {
    pub input_file_id: String,
    pub endpoint: String,
    pub completion_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// Batch response from the API.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResponse {
    pub id: String,
    pub object: String,
    pub endpoint: String,
    pub input_file_id: String,
    pub completion_window: String,
    pub status: String,
    #[serde(default)]
    pub output_file_id: Option<String>,
    #[serde(default)]
    pub error_file_id: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub in_progress_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub cancelled_at: Option<i64>,
    #[serde(default)]
    pub cancelling_at: Option<i64>,
    #[serde(default)]
    pub request_counts: Option<RequestCounts>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Request counts within a batch.
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestCounts {
    pub total: i64,
    pub completed: i64,
    pub failed: i64,
}

/// Parameters for listing batches.
#[derive(Debug, Default, Serialize)]
pub struct ListBatchesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_first: Option<bool>,
}

/// Batch list response (cursor pagination).
#[derive(Debug, Deserialize)]
pub struct BatchListResponse {
    #[serde(default)]
    pub object: String,
    pub data: Vec<BatchResponse>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub first_id: Option<String>,
    #[serde(default)]
    pub last_id: Option<String>,
}

impl BatchResponse {
    /// Whether the batch is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "completed" | "failed" | "cancelled" | "expired"
        )
    }

    /// Whether the batch is actively running.
    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "in_progress" | "validating")
    }

    /// Progress as a fraction (0.0 to 1.0), if request counts are available.
    pub fn progress(&self) -> Option<f64> {
        self.request_counts.as_ref().and_then(|rc| {
            if rc.total > 0 {
                Some((rc.completed + rc.failed) as f64 / rc.total as f64)
            } else {
                None
            }
        })
    }
}
