use std::borrow::Cow;
use std::collections::HashMap;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};

use bytes::Bytes;
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCategory {
    Unknown,
    Cancelled,
    InvalidArgument,
    FailedPrecondition,
    NotFound,
    PermissionDenied,
    Unimplemented,
}

impl ErrorCategory {
    pub fn to_status_code(self) -> StatusCode {
        match self {
            ErrorCategory::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCategory::Cancelled => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCategory::InvalidArgument => StatusCode::BAD_REQUEST,
            ErrorCategory::FailedPrecondition => StatusCode::BAD_REQUEST,
            ErrorCategory::NotFound => StatusCode::NOT_FOUND,
            ErrorCategory::PermissionDenied => StatusCode::FORBIDDEN,
            ErrorCategory::Unimplemented => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub category: ErrorCategory,
    pub error_code: Cow<'static, str>,
    pub instance_id: String,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn with_message(
        category: ErrorCategory,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            category,
            error_code: code.into(),
            instance_id: nanoid::nanoid!(),
            message: message.into(),
        }
    }

    pub fn unimplemented() -> Self {
        Self::with_message(ErrorCategory::Unimplemented, "unimplemented", "unimplemented")
    }

    pub fn unknown(message: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorCategory::Unknown, "unknown", message)
    }

    pub fn unavailable() -> Self {
        Self::with_message(ErrorCategory::Cancelled, "unavailable", "service unavailable")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status_code = self.category.to_status_code();
        (status_code, Json(self)).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLandmark {
    pub position: Vec3,
    pub presence: Option<f32>,
    pub visibility: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub struct Face {
    pub landmarks: Vec<FaceLandmark>,
    pub blend_shapes: HashMap<String, f32>,
    pub transform: Mat4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFacesRequest {
    pub faces: Vec<Face>,
}

#[derive(Debug, Clone)]
pub struct SetCameraRequest {
    pub width: u32,
    pub height: u32,
    pub payload: Bytes,
}
