use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Unauthorized - token may be expired")]
    Unauthorized,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Rate limited - please wait before retrying")]
    RateLimited,

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Maximum length for error response bodies in error messages
const MAX_ERROR_BODY_LENGTH: usize = 500;

impl ApiError {
    /// Truncate a response body to avoid logging excessive data
    fn truncate_body(body: &str) -> String {
        if body.len() <= MAX_ERROR_BODY_LENGTH {
            body.to_string()
        } else {
            format!("{}... (truncated, {} total bytes)",
                    &body[..MAX_ERROR_BODY_LENGTH],
                    body.len())
        }
    }

    pub fn from_status(status: reqwest::StatusCode, body: &str) -> Self {
        let truncated = Self::truncate_body(body);
        match status.as_u16() {
            401 => ApiError::Unauthorized,
            403 => ApiError::AccessDenied(truncated),
            404 => ApiError::NotFound(truncated),
            429 => ApiError::RateLimited,
            500..=599 => ApiError::ServerError(truncated),
            _ => ApiError::InvalidResponse(format!("Status {}: {}", status, truncated)),
        }
    }
}
