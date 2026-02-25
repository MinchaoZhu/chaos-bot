use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    NotFound,
    ServiceUnavailable,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::NotFound => "not_found",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Internal => "internal_error",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppError {
    code: ErrorCode,
    message: String,
    status: StatusCode,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ServiceUnavailable,
            message: message.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                code: self.code.as_str(),
                message: self.message,
            }),
        )
            .into_response()
    }
}
