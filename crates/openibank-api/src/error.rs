//! API error handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

impl From<openibank_types::OpeniBankError> for ApiError {
    fn from(err: openibank_types::OpeniBankError) -> Self {
        Self {
            code: err.error_code().to_string(),
            message: err.to_string(),
        }
    }
}
