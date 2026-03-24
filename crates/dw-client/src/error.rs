use serde::Deserialize;

/// API error response body from the Doubleword server.
#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub retry_after_seconds: Option<u64>,
}

/// Errors returned by the Doubleword API client.
#[derive(Debug, thiserror::Error)]
pub enum DwError {
    /// API returned an error response.
    #[error("{status} {error}: {message}")]
    Api {
        status: u16,
        error: String,
        message: String,
    },

    /// Authentication is missing or invalid.
    #[error("Not authenticated. Run `dw login` to authenticate.")]
    Unauthenticated,

    /// Insufficient permissions for the requested operation.
    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    /// The requested resource was not found.
    #[error("{resource} '{id}' not found")]
    NotFound { resource: String, id: String },

    /// Rate limited by the server.
    #[error("Rate limited{}", match .retry_after {
        Some(secs) => format!(". Retry after {}s", secs),
        None => String::new(),
    })]
    RateLimited { retry_after: Option<u64> },

    /// Request payload too large (e.g. file upload).
    #[error("Payload too large")]
    PayloadTooLarge,

    /// Network or connection error.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// IO error (file reading, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// No API key configured for the required endpoint type.
    #[error("No {key_type} API key configured. {hint}")]
    MissingKey { key_type: String, hint: String },
}

impl DwError {
    /// Whether this error is transient and worth retrying.
    ///
    /// Connection errors, timeouts, rate limits, and 5xx server errors are transient.
    /// Auth errors, 4xx client errors, decode errors, and request construction errors are permanent.
    pub fn is_transient(&self) -> bool {
        match self {
            DwError::RateLimited { .. } => true,
            DwError::Network(e) => e.is_timeout() || e.is_connect(),
            DwError::Api { status, .. } => *status >= 500,
            _ => false,
        }
    }

    /// Parse an HTTP response into a `DwError`.
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();

        // Read raw body text first, then try to parse as JSON.
        // The server returns JSON for some errors and plain text for others.
        let raw_body = response.text().await.unwrap_or_default();

        let body = serde_json::from_str::<ApiErrorBody>(&raw_body).unwrap_or(ApiErrorBody {
            error: None,
            message: None,
            retry_after_seconds: None,
        });

        let error = body
            .error
            .unwrap_or_else(|| status_to_string(status).to_string());

        // Use parsed JSON message, or fall back to raw body text
        let message = body.message.unwrap_or_else(|| {
            let trimmed = raw_body.trim();
            if trimmed.is_empty() {
                "No details provided".to_string()
            } else {
                trimmed.to_string()
            }
        });

        match status {
            401 => DwError::Unauthenticated,
            403 => DwError::Forbidden { message },
            404 => DwError::Api {
                status,
                error,
                message,
            },
            413 => DwError::PayloadTooLarge,
            429 => DwError::RateLimited {
                retry_after: body.retry_after_seconds,
            },
            _ => DwError::Api {
                status,
                error,
                message,
            },
        }
    }
}

fn status_to_string(status: u16) -> &'static str {
    match status {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Error",
    }
}
