use std::fmt;
use std::path::PathBuf;

use super::{get_env_var, Result};

/// Configuration for communicating with Steam.
pub struct Config {
	/// Steam [WebAPI] Key.
	///
	/// [WebAPI]: https://steamcommunity.com/dev
	pub api_key: String,

	/// Path to the steamcmd executable.
	pub steam_cmd_path: Option<PathBuf>,

	/// Path to the directory where workshop files should be downloaded to.
	pub steam_workshop_path: Option<PathBuf>,
}

impl Config {
	pub fn new() -> Result<Self> {
		let api_key = get_env_var("KZ_API_STEAM_API_KEY")?;
		let steam_cmd_path = get_env_var("KZ_API_STEAM_CMD_PATH").ok();
		let steam_workshop_path = get_env_var("KZ_API_STEAM_WORKSHOP_PATH").ok();

		Ok(Self { api_key, steam_cmd_path, steam_workshop_path })
	}
}

impl fmt::Debug for Config {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Steam Config")
			.field("api_key", &"…")
			.field("steamcmd", &self.steam_cmd_path)
			.field("workshop_path", &self.steam_workshop_path)
			.finish()
	}
}
