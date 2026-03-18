use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::files::{FileCostEstimate, FileListResponse, FileResponse, ListFilesParams};
use reqwest::multipart;
use std::path::Path;

impl DwClient {
    /// Upload a JSONL file for batch processing.
    ///
    /// Corresponds to `POST /v1/files`.
    pub async fn upload_file(&self, path: &Path, purpose: &str) -> Result<FileResponse, DwError> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("batch.jsonl")
            .to_string();

        let file_bytes = tokio::fs::read(path).await?;

        let file_part = multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("application/jsonl")
            .map_err(DwError::Network)?;

        let form = multipart::Form::new()
            .part("file", file_part)
            .text("purpose", purpose.to_string());

        let request = self.post(ApiSurface::Ai, "/v1/files")?.multipart(form);
        self.send(request).await
    }

    /// List uploaded files.
    ///
    /// Corresponds to `GET /v1/files`.
    pub async fn list_files(&self, params: &ListFilesParams) -> Result<FileListResponse, DwError> {
        let request = self.get(ApiSurface::Ai, "/v1/files")?.query(params);
        self.send(request).await
    }

    /// Get a specific file's metadata.
    ///
    /// Corresponds to `GET /v1/files/{file_id}`.
    pub async fn get_file(&self, file_id: &str) -> Result<FileResponse, DwError> {
        let request = self.get(ApiSurface::Ai, &format!("/v1/files/{}", file_id))?;
        self.send(request).await
    }

    /// Delete a file.
    ///
    /// Corresponds to `DELETE /v1/files/{file_id}`.
    pub async fn delete_file(&self, file_id: &str) -> Result<(), DwError> {
        let request = self.delete(ApiSurface::Ai, &format!("/v1/files/{}", file_id))?;
        self.send_empty(request).await
    }

    /// Get a file's content (raw bytes).
    ///
    /// Corresponds to `GET /v1/files/{file_id}/content`.
    pub async fn get_file_content(&self, file_id: &str) -> Result<bytes::Bytes, DwError> {
        let request = self.get(ApiSurface::Ai, &format!("/v1/files/{}/content", file_id))?;
        self.send_bytes(request).await
    }

    /// Get cost estimate for processing a file.
    ///
    /// Corresponds to `GET /v1/files/{file_id}/cost-estimate`.
    pub async fn get_file_cost_estimate(
        &self,
        file_id: &str,
        completion_window: Option<&str>,
    ) -> Result<FileCostEstimate, DwError> {
        let mut request = self.get(
            ApiSurface::Ai,
            &format!("/v1/files/{}/cost-estimate", file_id),
        )?;
        if let Some(window) = completion_window {
            request = request.query(&[("completion_window", window)]);
        }
        self.send(request).await
    }
}
