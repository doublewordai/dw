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
///
/// Always construct with `..Default::default()` to be forward-compatible with new fields:
/// ```ignore
/// DwClientConfig { ai_base_url: "...".into(), ..Default::default() }
/// ```
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
    /// Max retries on transient errors (429, 5xx, network). Default: 1.
    pub max_retries: u32,
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
            max_retries: 1,
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
    /// Retries on transient errors (429, 5xx, network) up to `config.max_retries` times.
    pub async fn send<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DwError> {
        let response = self.send_with_retry(request).await?;
        Ok(response.json().await?)
    }

    /// Send a request and parse the JSON response, without retries.
    /// Use this in polling loops that handle their own retry logic.
    /// On 429, extracts `Retry-After` from headers so callers can honor the delay.
    pub async fn send_once<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, DwError> {
        let response = request.send().await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = Self::extract_retry_after(response).await;
            Err(DwError::RateLimited {
                retry_after: Some(retry_after),
            })
        } else {
            Err(DwError::from_response(response).await)
        }
    }

    /// Send a request and return raw bytes (for file content downloads).
    /// Retries on transient errors (429, 5xx, network) up to `config.max_retries` times.
    pub async fn send_bytes(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<bytes::Bytes, DwError> {
        let response = self.send_with_retry(request).await?;
        Ok(response.bytes().await?)
    }

    /// Send a request expecting no response body (204, etc).
    /// Retries on transient errors (429, 5xx, network) up to `config.max_retries` times.
    pub async fn send_empty(&self, request: reqwest::RequestBuilder) -> Result<(), DwError> {
        self.send_with_retry(request).await?;
        Ok(())
    }

    /// Capped backoff delay: 2^(attempt+1) seconds, max 60s.
    fn backoff_delay(attempt: u32) -> Duration {
        let secs = 2u64.saturating_pow(attempt + 1).min(60);
        Duration::from_secs(secs)
    }

    /// Extract retry-after delay from a 429 response (header or body).
    /// Always consumes the response body to allow connection reuse.
    pub(crate) async fn extract_retry_after(response: reqwest::Response) -> u64 {
        let header_retry = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        if let Some(secs) = header_retry {
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
        }
    }

    /// Send a request, retrying on transient errors (429, 5xx, network).
    ///
    /// Uses `config.max_retries` (default: 1, clamped to max 10). Set to 0 to
    /// disable retries. On 429, extracts retry delay from the `Retry-After`
    /// header (integer seconds only — HTTP-date values are ignored and fall back
    /// to `retry_after_seconds` in the JSON body, or 30s default).
    /// On 5xx/network errors, uses exponential backoff (2s, 4s, 8s... capped at 60s).
    ///
    /// Retries only occur when `try_clone()` succeeds. Streamed/multipart bodies
    /// (e.g. file uploads) cannot be cloned and are never retried. Small JSON
    /// POST bodies can be cloned and may be retried — the server returns 409 on
    /// duplicate resource creation.
    async fn send_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, DwError> {
        let max_retries = self.config.max_retries.min(10);

        // Lazy cloning: keep at most one "next attempt" clone at a time
        let mut current = request;
        let mut attempt: u32 = 0;

        loop {
            // Clone lazily before sending — only if we might need to retry
            let next = if attempt < max_retries {
                current.try_clone()
            } else {
                None
            };

            let response = match current.send().await {
                Ok(r) => r,
                Err(e) => {
                    if let Some(retry) = next
                        && (e.is_timeout() || e.is_connect() || e.is_request())
                    {
                        let delay = Self::backoff_delay(attempt);
                        tracing::warn!(attempt, ?delay, "Network error, retrying: {}", e);
                        tokio::time::sleep(delay).await;
                        current = retry;
                        attempt += 1;
                        continue;
                    }
                    return Err(e.into());
                }
            };

            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = Self::extract_retry_after(response).await;
                if let Some(retry) = next {
                    tracing::warn!(attempt, retry_after, "Rate limited (429), retrying");
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                    current = retry;
                    attempt += 1;
                    continue;
                }
                return Err(DwError::RateLimited {
                    retry_after: Some(retry_after),
                });
            }

            if response.status().is_server_error()
                && let Some(retry) = next
            {
                let delay = Self::backoff_delay(attempt);
                tracing::warn!(
                    attempt,
                    ?delay,
                    "Server error ({}), retrying",
                    response.status()
                );
                let _ = response.bytes().await;
                tokio::time::sleep(delay).await;
                current = retry;
                attempt += 1;
                continue;
            }

            if response.status().is_success() {
                return Ok(response);
            }
            return Err(DwError::from_response(response).await);
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
