use serde::Deserialize;
use std::path::PathBuf;

/// Server configuration.
#[derive(Deserialize)]
pub struct Config {
	/// The HTTP server's port.
	pub port: u16,
	/// The URL to the Discord server's invitation.
	pub discord_invite: String,

	/// The path to articles.
	pub article_path: PathBuf,
	/// The path to article assets.
	pub article_assets_path: PathBuf,
}
