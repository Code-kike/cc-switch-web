use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};

pub type ApiResult<T> = Result<Json<T>, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    details: Option<Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }

    pub fn not_supported(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "WEB_NOT_SUPPORTED", message)
    }

    pub fn desktop_only(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "WEB_DESKTOP_ONLY", message)
    }

    pub fn upload_required(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "WEB_UPLOAD_REQUIRED", message)
    }

    pub fn from_anyhow<E: std::fmt::Display>(error: E) -> Self {
        Self::internal(error.to_string())
    }

    pub fn from_service_message(message: String) -> Self {
        if message.contains("unavailable in web-server mode")
            || message.contains("not supported in this runtime")
        {
            return Self::not_supported(message);
        }
        Self::bad_request(message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "code": self.code,
            "message": self.message,
            "details": self.details,
        });
        (self.status, Json(body)).into_response()
    }
}

pub fn json_ok<T: Serialize>(value: T) -> Json<T> {
    Json(value)
}

pub async fn web_not_supported() -> Result<Json<Value>, ApiError> {
    Err(ApiError::not_supported(
        "This command is not implemented in Web mode yet",
    ))
}

pub async fn web_desktop_only() -> Result<Json<Value>, ApiError> {
    Err(ApiError::desktop_only(
        "This desktop-only command is not available in Web mode",
    ))
}

pub async fn web_upload_required() -> Result<Json<Value>, ApiError> {
    Err(ApiError::upload_required(
        "Use the Web upload/download endpoint for this file operation",
    ))
}
