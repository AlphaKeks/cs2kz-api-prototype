use axum::{Extension, Json};
use chrono::{DateTime, Utc};

use crate::auth::servers::AuthenticatedServer;
use crate::auth::{Session, JWT};
use crate::bans::{CreatedBan, NewBan};
use crate::extractors::State;
use crate::responses::Created;
use crate::{audit_error, responses, Error, Result};

/// Ban a player.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  tag = "Bans",
  path = "/bans",
  responses(
    responses::Created<CreatedBan>,
    responses::Unauthorized,
    responses::Forbidden,
    responses::UnprocessableEntity,
    responses::InternalServerError,
  ),
  security(
    ("Steam Session" = ["bans_create"]),
    ("CS2 Server JWT" = []),
  ),
)]
pub async fn create(
	state: State,
	server: Option<JWT<AuthenticatedServer>>,
	admin: Option<Extension<Session>>,
	Json(ban): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	if server.is_none() && admin.is_none() {
		audit_error!(?ban, "ban submitted without authentication");
		return Err(Error::Unauthorized);
	}

	let (server_id, plugin_version_id) = server
		.map(|server| (server.id, server.plugin_version_id))
		.unzip();

	let banned_by = admin.map(|admin| admin.steam_id);

	// FIXME
	let expires_on = None::<DateTime<Utc>>;

	let mut transaction = state.transaction().await?;

	sqlx::query! {
		r#"
		INSERT INTO
		  Bans (
		    player_id,
		    player_ip,
		    reason,
		    server_id,
		    plugin_version_id,
		    banned_by,
		    expires_on
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?)
		"#,
		ban.steam_id,
		ban.ip_address.map(|addr| addr.to_string()),
		ban.reason,
		server_id,
		plugin_version_id,
		banned_by,
		expires_on,
	}
	.execute(transaction.as_mut())
	.await?;

	let ban_id = sqlx::query!("SELECT LAST_INSERT_ID() id")
		.fetch_one(transaction.as_mut())
		.await
		.map(|row| row.id as _)?;

	transaction.commit().await?;

	Ok(Created(Json(CreatedBan { ban_id })))
}
