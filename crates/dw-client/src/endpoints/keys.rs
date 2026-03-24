use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::keys::{ApiKeyResponse, CreateApiKeyRequest, PaginatedApiKeys};

impl DwClient {
    /// Create an API key for the current user.
    /// Corresponds to `POST /admin/api/v1/users/current/api-keys` (requires platform key).
    /// Returns the full key response including the secret (shown only once).
    pub async fn create_api_key(
        &self,
        request: &CreateApiKeyRequest,
    ) -> Result<ApiKeyResponse, DwError> {
        let req = self
            .post(ApiSurface::Admin, "/admin/api/v1/users/current/api-keys")?
            .json(request);
        self.send(req).await
    }

    /// List API keys for the current user.
    /// Corresponds to `GET /admin/api/v1/users/current/api-keys` (requires platform key).
    pub async fn list_api_keys(&self) -> Result<PaginatedApiKeys, DwError> {
        let req = self.get(
            ApiSurface::Admin,
            "/admin/api/v1/users/current/api-keys?limit=100",
        )?;
        self.send(req).await
    }

    /// Delete an API key.
    /// Corresponds to `DELETE /admin/api/v1/users/current/api-keys/{key_id}` (requires platform key).
    pub async fn delete_api_key(&self, key_id: &str) -> Result<(), DwError> {
        let req = self.delete(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/current/api-keys/{}", key_id),
        )?;
        self.send_empty(req).await
    }
}
