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
    status_code: u16,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: message.into(),
            status_code: 400,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: message.into(),
            status_code: 404,
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ServiceUnavailable,
            message: message.into(),
            status_code: 503,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: message.into(),
            status_code: 500,
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
