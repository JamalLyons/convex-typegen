//! Maps the runtime's typed `CallError` to HTTP responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use convex_typegen_runtime::CallError;
use serde::Serialize;

use crate::model::{CreateFailure, CreateFailureCode};

#[derive(Debug)]
pub struct ApiError
{
    status: StatusCode,
    message: String,
}

impl ApiError
{
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self
    {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn not_found() -> Self
    {
        Self::new(StatusCode::NOT_FOUND, "bookmark not found")
    }
}

#[derive(Serialize)]
struct ErrorBody
{
    error: String,
}

impl IntoResponse for ApiError
{
    fn into_response(self) -> Response
    {
        (self.status, Json(ErrorBody { error: self.message })).into_response()
    }
}

/// A `ConvexError` payload the function declared with `with_error`.
impl From<CreateFailure> for ApiError
{
    fn from(failure: CreateFailure) -> Self
    {
        match failure.code {
            CreateFailureCode::InvalidUrl => Self::new(StatusCode::BAD_REQUEST, "url must be an http(s) URL"),
        }
    }
}

/// A `ConvexError` from a function without a declared payload type. None of ours throw one.
impl From<convex::Value> for ApiError
{
    fn from(_: convex::Value) -> Self
    {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "unexpected application error")
    }
}

impl<E> From<CallError<E>> for ApiError
where
    E: Into<ApiError>,
{
    fn from(error: CallError<E>) -> Self
    {
        match error {
            CallError::Application { data: Ok(payload), .. } => payload.into(),
            // Convex rejected the arguments, e.g. a path segment that isn't a bookmarks id.
            CallError::Server { 0: message, .. } if message.contains("ArgumentValidationError") => {
                Self::new(StatusCode::BAD_REQUEST, "invalid bookmark id")
            }
            CallError::Transport { .. } => {
                eprintln!("convex: {error}");
                Self::new(StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
            }
            // Encode/Decode mean the bindings and the deployment disagree; Server is a bug.
            other => {
                eprintln!("convex: {other}");
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        }
    }
}
