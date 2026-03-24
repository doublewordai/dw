use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::usage::{BatchAnalytics, UsageResponse};

/// Normalize a date string to ISO 8601 with time component.
/// Accepts "2026-03-01" → "2026-03-01T00:00:00Z" or passes through if already full.
fn normalize_date(date: &str) -> String {
    if date.contains('T') {
        date.to_string()
    } else {
        format!("{}T00:00:00Z", date)
    }
}

impl DwClient {
    /// Get usage summary for the current user (or active org).
    ///
    /// Optionally filter by date range. Without dates, returns all-time usage.
    pub async fn get_usage(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<UsageResponse, DwError> {
        let mut request = self.get(ApiSurface::Admin, "/admin/api/v1/usage")?;
        let mut query_params: Vec<(&str, String)> = Vec::new();
        if let Some(start) = start_date {
            query_params.push(("start_date", normalize_date(start)));
        }
        if let Some(end) = end_date {
            query_params.push(("end_date", normalize_date(end)));
        }
        if !query_params.is_empty() {
            request = request.query(&query_params);
        }
        self.send(request).await
    }

    /// Get analytics for a specific batch (token counts, latency, cost).
    pub async fn get_batch_analytics(&self, batch_id: &str) -> Result<BatchAnalytics, DwError> {
        let request = self.get(
            ApiSurface::Ai,
            &format!("/v1/batches/{}/analytics", batch_id),
        )?;
        self.send(request).await
    }
}
