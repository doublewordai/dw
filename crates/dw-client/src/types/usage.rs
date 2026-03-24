use serde::{Deserialize, Serialize};

/// Per-model usage breakdown.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelBreakdownEntry {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// Cost as a decimal string to preserve precision (e.g. "0.028914550000000").
    pub cost: String,
    pub request_count: i64,
}

/// User batch usage response with overall metrics and per-model breakdown.
#[derive(Debug, Serialize, Deserialize)]
pub struct UsageResponse {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_request_count: i64,
    pub total_batch_count: i64,
    pub avg_requests_per_batch: f64,
    /// Total cost as a decimal string to preserve precision (e.g. "113.436959620000000").
    pub total_cost: String,
    /// Estimated cost if all tokens were charged at current realtime tariff rates.
    /// Decimal string for precision.
    pub estimated_realtime_cost: String,
    pub by_model: Vec<ModelBreakdownEntry>,
}

/// Batch analytics: token counts, latency, and cost for a single batch.
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchAnalytics {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    #[serde(default)]
    pub avg_duration_ms: Option<f64>,
    #[serde(default)]
    pub avg_ttfb_ms: Option<f64>,
    /// Total cost as a decimal string to preserve precision. None if pricing unavailable.
    #[serde(default)]
    pub total_cost: Option<String>,
}

/// A single analytics entry from the requests list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEntry {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub uri: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub status_code: Option<u16>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub prompt_tokens: Option<i64>,
    #[serde(default)]
    pub completion_tokens: Option<i64>,
    #[serde(default)]
    pub total_tokens: Option<i64>,
    #[serde(default)]
    pub response_type: Option<String>,
    #[serde(default)]
    pub fusillade_batch_id: Option<String>,
    /// Input price per token as decimal string.
    #[serde(default)]
    pub input_price_per_token: Option<String>,
    /// Output price per token as decimal string.
    #[serde(default)]
    pub output_price_per_token: Option<String>,
    #[serde(default)]
    pub custom_id: Option<String>,
}

/// Response containing a list of analytics entries.
#[derive(Debug, Serialize, Deserialize)]
pub struct ListAnalyticsResponse {
    pub entries: Vec<AnalyticsEntry>,
}

/// Query parameters for listing requests.
#[derive(Debug, Clone, Serialize)]
pub struct ListRequestsParams {
    pub limit: u64,
    pub skip: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "timestamp_after")]
    pub since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "timestamp_before")]
    pub until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "fusillade_batch_id")]
    pub batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

impl Default for ListRequestsParams {
    fn default() -> Self {
        Self {
            limit: 20,
            skip: 0,
            model: None,
            since: None,
            until: None,
            batch_id: None,
            status_code: None,
        }
    }
}
