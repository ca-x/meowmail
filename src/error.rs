use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    Unauthorized,
    #[error("request verification failed")]
    Csrf,
    #[error("session is locked")]
    Locked,
    #[error("permission denied")]
    Forbidden,
    #[error("too many requests")]
    RateLimited,
    #[error("resource not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("mail server operation failed: {0}")]
    Mail(String),
    #[error("AI service operation failed: {0}")]
    Ai(String),
    #[error("calendar service operation failed: {0}")]
    Calendar(String),
    #[error("internal error")]
    Internal(#[source] anyhow::Error),
}

impl AppError {
    pub fn internal(error: impl Into<anyhow::Error>) -> Self {
        Self::Internal(error.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "AUTHENTICATION_REQUIRED",
                self.to_string(),
            ),
            Self::Csrf => (
                StatusCode::FORBIDDEN,
                "REQUEST_VERIFICATION_FAILED",
                self.to_string(),
            ),
            Self::Locked => (StatusCode::LOCKED, "SESSION_LOCKED", self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                self.to_string(),
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            Self::Conflict => (StatusCode::CONFLICT, "CONFLICT", self.to_string()),
            Self::Validation(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                self.to_string(),
            ),
            Self::Mail(_) => (
                StatusCode::BAD_GATEWAY,
                "MAIL_SERVER_ERROR",
                "The mail server operation failed".to_owned(),
            ),
            Self::Ai(_) => (
                StatusCode::BAD_GATEWAY,
                "AI_SERVICE_ERROR",
                "The AI service request failed".to_owned(),
            ),
            Self::Calendar(_) => (
                StatusCode::BAD_GATEWAY,
                "CALENDAR_SERVICE_ERROR",
                "The calendar service request failed".to_owned(),
            ),
            Self::Internal(error) => {
                tracing::error!(error = ?error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "The request could not be completed".to_owned(),
                )
            }
        };
        (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody<'a> {
    code: &'a str,
    message: String,
}

impl From<sea_orm::DbErr> for AppError {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::internal(error)
    }
}
