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
        let mut url = "/admin/api/v1/usage".to_string();
        let mut params = Vec::new();
        if let Some(start) = start_date {
            params.push(format!("start_date={}", normalize_date(start)));
        }
        if let Some(end) = end_date {
            params.push(format!("end_date={}", normalize_date(end)));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let request = self.get(ApiSurface::Admin, &url)?;
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
