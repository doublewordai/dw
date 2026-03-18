use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::models::{ModelListResponse, ModelResponse};

impl DwClient {
    /// List available models, fetching all pages.
    ///
    /// Corresponds to `GET /admin/api/v1/models`.
    /// Requires a platform key.
    pub async fn list_models(&self) -> Result<Vec<ModelResponse>, DwError> {
        let mut all_models = Vec::new();
        let mut skip: i64 = 0;
        let limit: i64 = 100; // Max allowed by API

        loop {
            let req = self
                .get(ApiSurface::Admin, "/admin/api/v1/models")?
                .query(&[("skip", skip), ("limit", limit)]);
            let page: ModelListResponse = self.send(req).await?;

            let page_len = page.data.len() as i64;
            all_models.extend(page.data);

            if page_len < limit || all_models.len() as i64 >= page.total_count {
                break;
            }
            skip += page_len;
        }

        Ok(all_models)
    }

    /// Get a specific model by UUID.
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

    /// Find a model by alias (case-insensitive).
    ///
    /// Fetches all models and finds the one matching the alias.
    /// Requires a platform key.
    pub async fn find_model_by_alias(&self, alias: &str) -> Result<Option<ModelResponse>, DwError> {
        let models = self.list_models().await?;
        Ok(models
            .into_iter()
            .find(|m| m.alias.eq_ignore_ascii_case(alias)))
    }
}
