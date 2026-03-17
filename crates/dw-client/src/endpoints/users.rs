use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::users::CurrentUser;

impl DwClient {
    /// Get the currently authenticated user.
    ///
    /// Corresponds to `GET /admin/api/v1/users/current`.
    /// Requires a platform key.
    pub async fn get_current_user(&self) -> Result<CurrentUser, DwError> {
        let req = self.get(ApiSurface::Admin, "/admin/api/v1/users/current")?;
        self.send(req).await
    }
}
