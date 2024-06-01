use serde::Deserialize;

serde_with::with_prefix!(github_prefix "github_");

/// Github configuration.
#[derive(Deserialize)]
pub struct GithubConfig {
	/// The client ID of the Github application.
	pub client_id: String,
	/// The client secret of the Github application.
	pub client_secret: String,
}

/// Server configuration.
#[derive(Deserialize)]
pub struct Config {
	/// The HTTP server's port.
	pub port: u16,
	/// The connection string for the database.
	pub db: String,
	/// The secret key used to secure sessions.
	pub session_secret_key: String,
	/// The URL to the Discord server's invitation.
	pub discord_invite: String,

	/// Github configuration.
	#[serde(flatten, with = "github_prefix")]
	pub github: GithubConfig,
}
