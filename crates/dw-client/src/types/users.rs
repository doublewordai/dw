use serde::Deserialize;

/// Current user response from /users/current.
#[derive(Debug, Deserialize)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
    #[serde(default)]
    pub user_type: Option<String>,
    #[serde(default)]
    pub credit_balance: Option<f64>,
    #[serde(default)]
    pub active_organization: Option<String>,
    #[serde(default)]
    pub organizations: Option<Vec<UserOrganization>>,
}

/// Organization summary as seen from the user's perspective.
#[derive(Debug, Deserialize)]
pub struct UserOrganization {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}
