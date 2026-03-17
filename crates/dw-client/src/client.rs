use crate::error::DwError;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use std::time::Duration;

const DEFAULT_AI_BASE_URL: &str = "https://api.doubleword.ai";
const DEFAULT_ADMIN_BASE_URL: &str = "https://app.doubleword.ai";
const USER_AGENT: &str = "dw-cli";

/// Which API surface an endpoint targets.
#[derive(Debug, Clone, Copy)]
pub enum ApiSurface {
    /// AI/inference endpoints at api.doubleword.ai (/ai/v1/*)
    Ai,
    /// Admin/management endpoints at app.doubleword.ai (/admin/api/v1/*)
    Admin,
}

/// Configuration for the Doubleword API client.
#[derive(Debug, Clone)]
pub struct DwClientConfig {
    pub ai_base_url: String,
    pub admin_base_url: String,
    pub realtime_key: Option<String>,
    pub platform_key: Option<String>,
    pub cli_version: String,
    pub timeout: Duration,
}

impl Default for DwClientConfig {
    fn default() -> Self {
        Self {
            ai_base_url: DEFAULT_AI_BASE_URL.to_string(),
            admin_base_url: DEFAULT_ADMIN_BASE_URL.to_string(),
            realtime_key: None,
            platform_key: None,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            timeout: Duration::from_secs(300),
        }
    }
}

/// The Doubleword API client.
///
/// Wraps HTTP interactions with both the AI and Admin API surfaces.
/// Each method knows which surface (and therefore which base URL + API key)
/// to use for its endpoint.
#[derive(Debug, Clone)]
pub struct DwClient {
    http: reqwest::Client,
    config: DwClientConfig,
}

impl DwClient {
    /// Create a new client with the given configuration.
    pub fn new(config: DwClientConfig) -> Result<Self, DwError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "X-DW-CLI-Version",
            HeaderValue::from_str(&config.cli_version)
                .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
        );

        let http = reqwest::Client::builder()
            .user_agent(format!("{}/{}", USER_AGENT, config.cli_version))
            .default_headers(default_headers)
            .timeout(config.timeout)
            .build()?;

        Ok(Self { http, config })
    }

    /// Create a client configured with just a realtime key (e.g. for agent use).
    pub fn with_realtime_key(key: String) -> Result<Self, DwError> {
        Self::new(DwClientConfig {
            realtime_key: Some(key),
            ..Default::default()
        })
    }

    /// Get the base URL for a given API surface.
    pub fn base_url(&self, surface: ApiSurface) -> &str {
        match surface {
            ApiSurface::Ai => &self.config.ai_base_url,
            ApiSurface::Admin => &self.config.admin_base_url,
        }
    }

    /// Get the API key for a given surface, or return an error if missing.
    fn api_key(&self, surface: ApiSurface) -> Result<&str, DwError> {
        match surface {
            ApiSurface::Ai => self.config.realtime_key.as_deref().ok_or(DwError::MissingKey {
                key_type: "realtime".to_string(),
                hint: "Run `dw login` or `dw login --api-key <key>` to authenticate.".to_string(),
            }),
            ApiSurface::Admin => {
                self.config
                    .platform_key
                    .as_deref()
                    .ok_or(DwError::MissingKey {
                        key_type: "platform".to_string(),
                        hint: "This command requires full authentication. Run `dw login` (browser flow) to get a platform key.".to_string(),
                    })
            }
        }
    }

    /// Build a GET request to the given path on the specified surface.
    pub fn get(&self, surface: ApiSurface, path: &str) -> Result<reqwest::RequestBuilder, DwError> {
        let url = format!("{}{}", self.base_url(surface), path);
        let key = self.api_key(surface)?;
        Ok(self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", key)))
    }

    /// Build a POST request to the given path on the specified surface.
    pub fn post(
        &self,
        surface: ApiSurface,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, DwError> {
        let url = format!("{}{}", self.base_url(surface), path);
        let key = self.api_key(surface)?;
        Ok(self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", key)))
    }

    /// Build a DELETE request to the given path on the specified surface.
    pub fn delete(
        &self,
        surface: ApiSurface,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, DwError> {
        let url = format!("{}{}", self.base_url(surface), path);
        let key = self.api_key(surface)?;
        Ok(self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", key)))
    }

    /// Build a PATCH request to the given path on the specified surface.
    pub fn patch(
        &self,
        surface: ApiSurface,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, DwError> {
        let url = format!("{}{}", self.base_url(surface), path);
        let key = self.api_key(surface)?;
        Ok(self
            .http
            .patch(&url)
            .header(AUTHORIZATION, format!("Bearer {}", key)))
    }

    /// Send a request and parse the JSON response, handling errors.
    pub async fn send<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DwError> {
        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(DwError::from_response(response).await)
        }
    }

    /// Send a request and return raw bytes (for file content downloads).
    pub async fn send_bytes(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<bytes::Bytes, DwError> {
        let response = request.send().await?;

        if response.status().is_success() {
            Ok(response.bytes().await?)
        } else {
            Err(DwError::from_response(response).await)
        }
    }

    /// Send a request expecting no response body (204, etc).
    pub async fn send_empty(&self, request: reqwest::RequestBuilder) -> Result<(), DwError> {
        let response = request.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(DwError::from_response(response).await)
        }
    }

    /// Get a reference to the underlying reqwest client (for streaming, etc).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Get the config.
    pub fn config(&self) -> &DwClientConfig {
        &self.config
    }

    /// Check if a platform key is configured.
    pub fn has_platform_key(&self) -> bool {
        self.config.platform_key.is_some()
    }

    /// Check if a realtime key is configured.
    pub fn has_realtime_key(&self) -> bool {
        self.config.realtime_key.is_some()
    }
}
