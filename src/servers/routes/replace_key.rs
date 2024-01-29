use axum::extract::Path;
use axum::Json;

use crate::responses::Created;
use crate::servers::CreatedServer;
use crate::{audit, responses, AppState, Error, Result};

/// Replace the key for a specific server with a new, random, one.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  tag = "Servers",
  path = "/servers/{server_id}/key",
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::Created<CreatedServer>,
    responses::Unauthorized,
    responses::Forbidden,
    responses::BadRequest,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["servers"]),
  ),
)]
pub async fn replace_key(
	state: AppState,
	Path(server_id): Path<u16>,
) -> Result<Created<Json<CreatedServer>>> {
	let api_key = rand::random::<u32>();

	let result = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  api_key = ?
		WHERE
		  id = ?
		"#,
		api_key,
		server_id,
	}
	.execute(state.database())
	.await?;

	if result.rows_affected() == 0 {
		return Err(Error::InvalidServerID(server_id));
	}

	audit!("updated API key for server", id = %server_id, new_key = %api_key);

	Ok(Created(Json(CreatedServer { server_id, api_key })))
}
