use {
	crate::{
		res::{servers as res, BadRequest},
		util::{self, Created, Filter, Limit, Offset},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

static ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		s.id,
		s.name,
		p.name player_name,
		p.id steam_id,
		s.ip_address,
		s.port
	FROM
		Servers s
		JOIN Players p ON p.id = s.owned_by
"#;

/// Query parameters for fetching servers.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetServersParams<'a> {
	/// A server name.
	name: Option<String>,

	/// `SteamID` or name of a player.
	owned_by: Option<PlayerIdentifier<'a>>,

	/// Only include servers that were approved after a certain date.
	created_after: Option<DateTime<Utc>>,

	/// Only include servers that were approved before a certain date.
	created_before: Option<DateTime<Utc>>,

	offset: Offset,
	limit: Limit<500>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers",
	params(GetServersParams),
	responses(
		(status = 200, body = Vec<Server>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_servers(
	state: State,
	Query(GetServersParams { name, owned_by, created_after, created_before, offset, limit }): Query<
		GetServersParams<'_>,
	>,
) -> Response<Vec<res::Server>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" s.name LIKE ")
			.push_bind(name);

		filter.switch();
	}

	if let Some(player) = owned_by {
		let steam32_id = match player {
			PlayerIdentifier::SteamID(steam_id) => steam_id.as_u32(),
			PlayerIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Players WHERE name LIKE ?", name)
					.fetch_one(state.database())
					.await?
					.id
			}
		};

		query
			.push(filter)
			.push(" p.id = ")
			.push_bind(steam32_id);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" s.approved_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" s.approved_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	util::push_limit(&mut query, offset, limit);

	let servers = query
		.build_query_as::<res::Server>()
		.fetch_all(state.database())
		.await?;

	if servers.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(servers))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Servers", context_path = "/api/v0", path = "/servers/{ident}",
	params(("ident" = ServerIdentifier, Path, description = "The servers's ID or name")),
	responses(
		(status = 200, body = Server),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_server(
	state: State,
	Path(ident): Path<ServerIdentifier<'_>>,
) -> Response<res::Server> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		ServerIdentifier::ID(id) => {
			query.push(" s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query
				.push(" s.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let server = query
		.build_query_as::<res::Server>()
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::NoContent)?;

	Ok(Json(server))
}

/// Information about a new KZ server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewServer {
	/// The name of the server.
	name: String,

	/// The `SteamID` of the player who owns this server.
	owned_by: SteamID,

	/// The IP address of this server.
	#[schema(value_type = String)]
	ip_address: Ipv4Addr,

	/// The port of this server.
	port: u16,

	/// The `SteamID` of the admin who approved this server.
	approved_by: SteamID,
}

/// Information about a newly created KZ server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedServer {
	/// The ID of the server.
	id: u16,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Servers", context_path = "/api/v0", path = "/servers",
	request_body = NewServer,
	responses(
		(status = 201, body = CreatedServer),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_server(
	state: State,
	Json(NewServer { name, owned_by, ip_address, port, approved_by }): Json<NewServer>,
) -> Result<Created<Json<CreatedServer>>> {
	let api_key = rand::random::<u32>();
	let mut transaction = state.database().begin().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Servers (name, ip_address, port, owned_by, api_key)
		VALUES
			(?, ?, ?, ?, ?)
		"#,
		name,
		ip_address.to_string(),
		port,
		owned_by.as_u32(),
		api_key,
	}
	.execute(transaction.as_mut())
	.await?;

	let id = sqlx::query!("SELECT MAX(id) id FROM Servers")
		.fetch_one(transaction.as_mut())
		.await?
		.id
		.expect("server was just inserted");

	transaction.commit().await?;

	Ok(Created(Json(CreatedServer { id })))
}

/// Updated information about a KZ server.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// The new name of the server.
	name: Option<String>,

	/// The `SteamID` of the new server owner.
	owned_by: Option<SteamID>,

	/// The new IP address of the server.
	#[schema(value_type = Option<String>)]
	ip_address: Option<Ipv4Addr>,

	/// The new port of the server.
	port: Option<u16>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Servers", context_path = "/api/v0", path = "/servers/{id}",
	params(("id" = u16, Path, description = "The server's ID")),
	request_body = ServerUpdate,
	responses(
		(status = 200),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn update_server(
	state: State,
	Path(server_id): Path<u16>,
	Json(ServerUpdate { name, owned_by, ip_address, port }): Json<ServerUpdate>,
) -> Result<()> {
	let mut transaction = state.database().begin().await?;

	if let Some(name) = name {
		sqlx::query!("UPDATE Servers SET name = ? WHERE id = ?", name, server_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(steam_id) = owned_by {
		sqlx::query!("UPDATE Servers SET owned_by = ? WHERE id = ?", steam_id.as_u32(), server_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(ip_address) = ip_address.map(|ip| ip.to_string()) {
		sqlx::query!("UPDATE Servers SET ip_address = ? WHERE id = ?", ip_address, server_id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(port) = port {
		sqlx::query!("UPDATE Servers SET port = ? WHERE id = ?", port, server_id)
			.execute(transaction.as_mut())
			.await?;
	}

	transaction.commit().await?;

	Ok(())
}
