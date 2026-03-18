use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::models::{ModelListResponse, ModelResponse};

impl DwClient {
    /// List available models.
    ///
    /// Corresponds to `GET /admin/api/v1/models`.
    /// Requires a platform key.
    pub async fn list_models(&self) -> Result<ModelListResponse, DwError> {
        let req = self.get(ApiSurface::Admin, "/admin/api/v1/models")?;
        self.send(req).await
    }

    /// Get a specific model.
    ///
    /// Corresponds to `GET /admin/api/v1/models/{model_id}`.
    /// Requires a platform key.
    pub async fn get_model(&self, model_id: &str) -> Result<ModelResponse, DwError> {
        let req = self.get(
            ApiSurface::Admin,
            &format!("/admin/api/v1/models/{}", model_id),
        )?;
        self.send(req).await
    }
}
