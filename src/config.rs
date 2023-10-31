use serde::Deserialize;

/// S3 configuration for files storage.
#[derive(Deserialize)]
pub struct S3Config {
	/// The bucket's region.
	pub region: String,
	/// The endpoint of the service.
	pub endpoint: String,
	/// The bucket's name.
	pub bucket: String,
}

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

	/// s3 configuration.
	pub s3: S3Config,
	/// Github configuration.
	pub github: GithubConfig,
}
