use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::keys::ApiKeyResponse;

impl DwClient {
    /// List API keys for a user.
    ///
    /// Corresponds to `GET /admin/api/v1/users/{user_id}/api-keys`.
    /// Requires a platform key.
    pub async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyResponse>, DwError> {
        let req = self.get(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/{}/api-keys", user_id),
        )?;
        self.send(req).await
    }

    /// Delete an API key.
    ///
    /// Corresponds to `DELETE /admin/api/v1/users/{user_id}/api-keys/{key_id}`.
    /// Requires a platform key.
    pub async fn delete_api_key(&self, user_id: &str, key_id: &str) -> Result<(), DwError> {
        let req = self.delete(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/{}/api-keys/{}", user_id, key_id),
        )?;
        self.send_empty(req).await
    }
}
