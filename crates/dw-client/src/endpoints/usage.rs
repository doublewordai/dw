use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::usage::{
    BatchAnalytics, ListAnalyticsResponse, ListRequestsParams, UsageResponse,
};

/// Append a UTC time component if the input looks like a bare date (YYYY-MM-DD only).
/// Passes through anything that already contains a time component ('T' or ':').
/// Does not validate the format — invalid strings are forwarded to the API as-is.
fn normalize_date(date: &str) -> String {
    if date.contains('T') || date.contains(':') {
        date.to_string()
    } else {
        format!("{}T00:00:00Z", date)
    }
}

impl DwClient {
    /// Get usage summary for the current user (or active org).
    /// Corresponds to `GET /admin/api/v1/usage` (requires platform key).
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

    /// List recent requests with filtering and pagination.
    /// Corresponds to `GET /admin/api/v1/requests` (requires platform key + RequestViewer role).
    pub async fn list_requests(
        &self,
        params: &ListRequestsParams,
    ) -> Result<ListAnalyticsResponse, DwError> {
        // Normalize date fields before sending
        let mut normalized = params.clone();
        if let Some(ref since) = normalized.since {
            normalized.since = Some(normalize_date(since));
        }
        if let Some(ref until) = normalized.until {
            normalized.until = Some(normalize_date(until));
        }

        let request = self
            .get(ApiSurface::Admin, "/admin/api/v1/requests")?
            .query(&normalized);
        self.send(request).await
    }

    /// Get analytics for a specific batch (token counts, latency, cost).
    /// Corresponds to `GET /v1/batches/{batch_id}/analytics` (requires inference key).
    pub async fn get_batch_analytics(&self, batch_id: &str) -> Result<BatchAnalytics, DwError> {
        let request = self.get(
            ApiSurface::Ai,
            &format!("/v1/batches/{}/analytics", batch_id),
        )?;
        self.send(request).await
    }
}
