pub mod health;
pub mod images;
pub mod openai;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{config::AppConfig, error::AppError, upstream::UpstreamClient};

#[derive(Clone)]
pub struct ApiState {
    pub config: AppConfig,
    pub upstream: Option<UpstreamClient>,
}

impl ApiState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            upstream: None,
        }
    }

    pub fn with_upstream(config: AppConfig, upstream: UpstreamClient) -> Self {
        Self {
            config,
            upstream: Some(upstream),
        }
    }

    pub fn upstream(&self) -> Result<UpstreamClient, ApiError> {
        self.upstream
            .clone()
            .ok_or_else(|| ApiError::unauthorized("login required"))
    }
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_IMPLEMENTED,
            code: "unsupported",
            message: message.into(),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::InvalidRequest(message) => ApiError::bad_request(message),
            AppError::Auth(message) => ApiError::unauthorized(message),
            other => Self {
                status: StatusCode::BAD_GATEWAY,
                code: "upstream_error",
                message: other.to_string(),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": {
                    "message": self.message,
                    "type": "invalid_request_error",
                    "code": self.code,
                }
            })),
        )
            .into_response()
    }
}
