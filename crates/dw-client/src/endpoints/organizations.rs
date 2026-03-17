use crate::client::{ApiSurface, DwClient};
use crate::error::DwError;
use crate::types::users::UserOrganization;

impl DwClient {
    /// List organizations the current user belongs to.
    ///
    /// Corresponds to `GET /admin/api/v1/users/{user_id}/organizations`.
    pub async fn list_user_organizations(
        &self,
        user_id: &str,
    ) -> Result<Vec<UserOrganization>, DwError> {
        let req = self.get(
            ApiSurface::Admin,
            &format!("/admin/api/v1/users/{}/organizations", user_id),
        )?;
        self.send(req).await
    }
}
