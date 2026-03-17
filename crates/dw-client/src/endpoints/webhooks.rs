use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::webhooks::{CreateWebhookRequest, WebhookResponse, WebhookWithSecretResponse};

impl DwClient {
    /// Create a webhook.
    ///
    /// Corresponds to `POST /admin/api/v1/users/{user_id}/webhooks`.
    pub async fn create_webhook(
        &self,
        user_id: &str,
        request: &CreateWebhookRequest,
    ) -> Result<WebhookWithSecretResponse, DwError> {
        let req = self
            .post(
                ApiSurface::Admin,
                &format!("/admin/api/v1/users/{}/webhooks", user_id),
            )?
            .json(request);
        self.send(req).await
    }

    /// List webhooks for a user.
    ///
    /// Corresponds to `GET /admin/api/v1/users/{user_id}/webhooks`.
    pub async fn list_webhooks(&self, user_id: &str) -> Result<Vec<WebhookResponse>, DwError> {
        let req = self.get(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/{}/webhooks", user_id),
        )?;
        self.send(req).await
    }

    /// Delete a webhook.
    ///
    /// Corresponds to `DELETE /admin/api/v1/users/{user_id}/webhooks/{webhook_id}`.
    pub async fn delete_webhook(&self, user_id: &str, webhook_id: &str) -> Result<(), DwError> {
        let req = self.delete(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/{}/webhooks/{}", user_id, webhook_id),
        )?;
        self.send_empty(req).await
    }

    /// Rotate a webhook's signing secret.
    ///
    /// Corresponds to `POST /admin/api/v1/users/{user_id}/webhooks/{webhook_id}/rotate-secret`.
    pub async fn rotate_webhook_secret(
        &self,
        user_id: &str,
        webhook_id: &str,
    ) -> Result<WebhookWithSecretResponse, DwError> {
        let req = self.post(
            ApiSurface::Admin,
            &format!(
                "/admin/api/v1/users/{}/webhooks/{}/rotate-secret",
                user_id, webhook_id
            ),
        )?;
        self.send(req).await
    }
}
