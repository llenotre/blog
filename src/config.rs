use serde::Deserialize;

serde_with::with_prefix!(github_prefix "github_");

/// Server configuration.
#[derive(Deserialize)]
pub struct Config {
	/// The HTTP server's port.
	pub port: u16,
	/// The URL to the Discord server's invitation.
	pub discord_invite: String,
}
