use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::batches::{
    BatchListResponse, BatchResponse, CreateBatchRequest, ListBatchesParams,
};

impl DwClient {
    /// Create a new batch.
    ///
    /// Corresponds to `POST /v1/batches`.
    pub async fn create_batch(
        &self,
        request: &CreateBatchRequest,
    ) -> Result<BatchResponse, DwError> {
        let req = self.post(ApiSurface::Ai, "/v1/batches")?.json(request);
        self.send(req).await
    }

    /// List batches.
    ///
    /// Corresponds to `GET /v1/batches`.
    pub async fn list_batches(
        &self,
        params: &ListBatchesParams,
    ) -> Result<BatchListResponse, DwError> {
        let req = self.get(ApiSurface::Ai, "/v1/batches")?.query(params);
        self.send(req).await
    }

    /// Get a specific batch.
    ///
    /// Corresponds to `GET /v1/batches/{batch_id}`.
    pub async fn get_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.get(ApiSurface::Ai, &format!("/v1/batches/{}", batch_id))?;
        self.send(req).await
    }

    /// Get batch details without client-level retries.
    /// Use this in polling loops that handle their own retry logic.
    pub async fn get_batch_once(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.get(ApiSurface::Ai, &format!("/v1/batches/{}", batch_id))?;
        self.send_once(req).await
    }

    /// Cancel a batch.
    ///
    /// Corresponds to `POST /v1/batches/{batch_id}/cancel`.
    pub async fn cancel_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.post(ApiSurface::Ai, &format!("/v1/batches/{}/cancel", batch_id))?;
        self.send(req).await
    }

    /// Retry failed requests in a batch.
    ///
    /// Corresponds to `POST /v1/batches/{batch_id}/retry`.
    pub async fn retry_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.post(ApiSurface::Ai, &format!("/v1/batches/{}/retry", batch_id))?;
        self.send(req).await
    }

    /// Get batch results as raw bytes (JSONL content).
    ///
    /// Fetches the output file content for a completed batch.
    pub async fn get_batch_results(&self, batch_id: &str) -> Result<bytes::Bytes, DwError> {
        // First get the batch to find the output file ID
        let batch = self.get_batch(batch_id).await?;

        let output_file_id = batch.output_file_id.ok_or_else(|| DwError::Api {
            status: 400,
            error: "No Results".to_string(),
            message: format!(
                "Batch {} has no output file (status: {})",
                batch_id, batch.status
            ),
        })?;

        self.get_file_content(&output_file_id).await
    }

    /// Fetch a page of batch results with pagination.
    ///
    /// Returns the JSONL body, whether more results are available (`X-Incomplete`),
    /// and the offset of the last line returned (`X-Last-Line`).
    ///
    /// Corresponds to `GET /v1/batches/{batch_id}/results?skip=N&limit=M&status=S`.
    ///
    /// Does not use client-level retries (`send_with_retry`) — callers in polling
    /// loops handle their own retry logic. On 429, returns `DwError::RateLimited`
    /// with a retry delay (server-provided when available, otherwise a default).
    pub async fn get_batch_results_page(
        &self,
        batch_id: &str,
        skip: usize,
        limit: usize,
        status: Option<&str>,
    ) -> Result<BatchResultsPage, DwError> {
        let mut query_params: Vec<(&str, String)> =
            vec![("skip", skip.to_string()), ("limit", limit.to_string())];
        if let Some(s) = status {
            query_params.push(("status", s.to_string()));
        }

        let request = self
            .get(ApiSurface::Ai, &format!("/v1/batches/{}/results", batch_id))?
            .query(&query_params);

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = DwClient::extract_retry_after(response).await;
            return Err(DwError::RateLimited {
                retry_after: Some(retry_after),
            });
        }

        if !response.status().is_success() {
            return Err(DwError::from_response(response).await);
        }

        let incomplete = response
            .headers()
            .get("x-incomplete")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v == "true");

        let last_line = response
            .headers()
            .get("x-last-line")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(skip);

        let body = response.text().await?;

        Ok(BatchResultsPage {
            body,
            incomplete,
            last_line,
        })
    }
}

/// A page of batch results from the paginated results endpoint.
pub struct BatchResultsPage {
    /// The JSONL content for this page.
    pub body: String,
    /// Whether more results are available (more pages or batch still processing).
    pub incomplete: bool,
    /// The offset of the last line returned.
    pub last_line: usize,
}
