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
