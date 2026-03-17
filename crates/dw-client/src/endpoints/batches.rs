use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::batches::{
    BatchListResponse, BatchResponse, CreateBatchRequest, ListBatchesParams,
};

impl DwClient {
    /// Create a new batch.
    ///
    /// Corresponds to `POST /ai/v1/batches`.
    pub async fn create_batch(
        &self,
        request: &CreateBatchRequest,
    ) -> Result<BatchResponse, DwError> {
        let req = self.post(ApiSurface::Ai, "/ai/v1/batches")?.json(request);
        self.send(req).await
    }

    /// List batches.
    ///
    /// Corresponds to `GET /ai/v1/batches`.
    pub async fn list_batches(
        &self,
        params: &ListBatchesParams,
    ) -> Result<BatchListResponse, DwError> {
        let req = self.get(ApiSurface::Ai, "/ai/v1/batches")?.query(params);
        self.send(req).await
    }

    /// Get a specific batch.
    ///
    /// Corresponds to `GET /ai/v1/batches/{batch_id}`.
    pub async fn get_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.get(ApiSurface::Ai, &format!("/ai/v1/batches/{}", batch_id))?;
        self.send(req).await
    }

    /// Cancel a batch.
    ///
    /// Corresponds to `POST /ai/v1/batches/{batch_id}/cancel`.
    pub async fn cancel_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.post(
            ApiSurface::Ai,
            &format!("/ai/v1/batches/{}/cancel", batch_id),
        )?;
        self.send(req).await
    }

    /// Retry failed requests in a batch.
    ///
    /// Corresponds to `POST /ai/v1/batches/{batch_id}/retry`.
    pub async fn retry_batch(&self, batch_id: &str) -> Result<BatchResponse, DwError> {
        let req = self.post(
            ApiSurface::Ai,
            &format!("/ai/v1/batches/{}/retry", batch_id),
        )?;
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
}
