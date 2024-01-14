use std::result::Result as StdResult;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value as JsonValue};
use thiserror::Error as ThisError;

use crate::auth::permissions::Permissions;

pub type Result<T> = StdResult<T, Error>;

/// Any errors that can occurr while middleware functions are executing.
#[derive(Debug, ThisError)]
pub enum Error {
	#[error("Request body could not be read as raw bytes: {0}")]
	InvalidRequestBody(axum::Error),

	#[error("You have insufficient permissions to make this request.")]
	InsufficientPermissions { required: Permissions },
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let mut json = json!({ "message": self.to_string() });
		let code = match self {
			Error::InvalidRequestBody(_) => StatusCode::BAD_REQUEST,
			Error::InsufficientPermissions { required } => {
				json["required_permissions"] = JsonValue::Number(required.0.into());
				StatusCode::FORBIDDEN
			}
		};

		(code, Json(json)).into_response()
	}
}