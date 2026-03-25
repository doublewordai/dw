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

    /// Fetch file content from an offset, returning new content and pagination state.
    ///
    /// Corresponds to `GET /v1/files/{file_id}/content?offset={offset}`.
    /// The server uses `X-Last-Line` to indicate the next offset and `X-Incomplete`
    /// to signal whether more content may follow.
    ///
    /// Returns `FileContentChunk::NotReady` on 404 (output file not yet created).
    /// Used for streaming batch results as they complete.
    pub async fn get_file_content_stream(
        &self,
        file_id: &str,
        offset: usize,
    ) -> Result<crate::types::files::FileContentChunk, DwError> {
        use crate::types::files::FileContentChunk;

        let mut url = format!("/v1/files/{}/content", file_id);
        if offset > 0 {
            url = format!("{}?offset={}", url, offset);
        }

        let request = self.get(ApiSurface::Ai, &url)?;
        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            if offset == 0 {
                // File not created yet — normal during early polling
                return Ok(FileContentChunk::NotReady);
            }
            // 404 after we've already read data = file deleted or wrong ID
            return Err(DwError::from_response(response).await);
        }

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

        let next_offset = response
            .headers()
            .get("x-last-line")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(offset);

        let body = response.text().await?;

        Ok(FileContentChunk::Data {
            body,
            next_offset,
            incomplete,
        })
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
