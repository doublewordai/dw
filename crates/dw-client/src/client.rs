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
    pub inference_key: Option<String>,
    pub platform_key: Option<String>,
    pub cli_version: String,
    /// Total request timeout. Default: 300s (5 minutes).
    pub timeout: Duration,
    /// TCP connect timeout. Default: 10s.
    pub connect_timeout: Duration,
}

impl Default for DwClientConfig {
    fn default() -> Self {
        Self {
            ai_base_url: DEFAULT_AI_BASE_URL.to_string(),
            admin_base_url: DEFAULT_ADMIN_BASE_URL.to_string(),
            inference_key: None,
            platform_key: None,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            timeout: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(10),
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
            .connect_timeout(config.connect_timeout)
            .build()?;

        Ok(Self { http, config })
    }

    /// Create a client configured with just an inference key (e.g. for agent use).
    pub fn with_inference_key(key: String) -> Result<Self, DwError> {
        Self::new(DwClientConfig {
            inference_key: Some(key),
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
            ApiSurface::Ai => self.config.inference_key.as_deref().ok_or(DwError::MissingKey {
                key_type: "inference".to_string(),
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
    /// Automatically retries once on 429 (rate limited) after the specified delay.
    pub async fn send<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DwError> {
        let response = self.send_with_retry(request).await?;
        Ok(response.json().await?)
    }

    /// Send a request and return raw bytes (for file content downloads).
    /// Automatically retries once on 429.
    pub async fn send_bytes(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<bytes::Bytes, DwError> {
        let response = self.send_with_retry(request).await?;
        Ok(response.bytes().await?)
    }

    /// Send a request expecting no response body (204, etc).
    /// Automatically retries once on 429.
    pub async fn send_empty(&self, request: reqwest::RequestBuilder) -> Result<(), DwError> {
        self.send_with_retry(request).await?;
        Ok(())
    }

    /// Send a request, retrying once on 429 (rate limited).
    ///
    /// On 429, extracts the retry delay from the `Retry-After` header (integer
    /// seconds only; HTTP-date values fall back to the default) or the
    /// `retry_after_seconds` field in the JSON response body. Defaults to 30s
    /// if neither is present.
    async fn send_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, DwError> {
        // Try to clone the request for potential retry
        let retry_request = request.try_clone();

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Extract retry delay from Retry-After header (integer seconds)
            let header_retry = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            // If no valid header, try parsing the body for retry_after_seconds.
            // Always consume the body to allow connection reuse in the pool.
            let retry_after = if let Some(secs) = header_retry {
                // Drain the body even though we already have the delay from headers
                let _ = response.bytes().await;
                secs
            } else {
                let body = response
                    .json::<crate::error::ApiErrorBody>()
                    .await
                    .unwrap_or(crate::error::ApiErrorBody {
                        error: None,
                        message: None,
                        retry_after_seconds: None,
                    });
                body.retry_after_seconds.unwrap_or(30)
            };

            tracing::warn!(retry_after, "Rate limited (429), retrying");

            tokio::time::sleep(Duration::from_secs(retry_after)).await;

            // Retry if we could clone the request
            if let Some(retry) = retry_request {
                let response = retry.send().await?;
                if response.status().is_success() {
                    return Ok(response);
                }
                return Err(DwError::from_response(response).await);
            }

            // Can't retry (request body was streamed) — return the rate limit error
            return Err(DwError::RateLimited {
                retry_after: Some(retry_after),
            });
        }

        if response.status().is_success() {
            Ok(response)
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

    /// Check if an inference key is configured.
    pub fn has_inference_key(&self) -> bool {
        self.config.inference_key.is_some()
    }
}
