//! This module implements user accounts.

use crate::config::GithubConfig;
use crate::util::Oid;
use crate::util::{FromRow, PgResult};
use actix_session::Session;
use actix_web::web::Redirect;
use macros::FromRow;
use serde::Deserialize;
use serde::Serialize;

/// The user agent for Github requests.
const GITHUB_USER_AGENT: &str = "maestro";
/// The Github API version.
const GITHUB_API_VERSION: &str = "2022-11-28";

// TODO update users from github data from time to time

/// Returns the authentication URL.
pub fn get_auth_url(client_id: &str) -> String {
	format!("https://github.com/login/oauth/authorize?client_id={client_id}")
}

/// Creates a session for the given user.
pub fn create_session(session: &Session, user: &User) -> actix_web::Result<()> {
	let insert_fields = || {
		session.insert("user_id", user.id)?;
		session.insert("user_login", &user.github_login)?;
		Ok(())
	};
	if insert_fields().is_err() {
		// Delete session and retry
		session.purge();
		insert_fields()
	} else {
		Ok(())
	}
}

/// Returns a redirection to the last article consulted by the session's user.
pub fn redirect_to_last_article(session: &Session) -> Redirect {
	let last_article = session.get::<String>("last_article").ok().flatten();
	match last_article {
		Some(slug) => Redirect::to(format!("/a/{slug}")),
		None => Redirect::to("/"),
	}
	.see_other()
}

/// Payload describing the Github access token for a user.
#[derive(Deserialize)]
pub struct GithubToken {
	/// The access token.
	access_token: Option<String>,
}

/// Payload describing a user on Github.
#[derive(Clone, Deserialize, Serialize)]
pub struct GithubUser {
	/// The user's login.
	pub login: String,
	/// The user's ID.
	pub id: i64,
	/// The URL to the user's profile.
	pub html_url: String,
}

/// A user, who can post comments, or if admin, administrate the website.
#[derive(Clone, FromRow)]
pub struct User {
	/// The user's id.
	pub id: Oid,

	/// The user's Github access token.
	pub access_token: String,
	/// The user's login.
	pub github_login: String,
	/// The user's ID.
	pub github_id: i64,
	/// Tells whether the user is admin.
	pub admin: bool,
}

impl User {
	/// Queries the access token from the given `code` returned by Github.
	pub async fn query_access_token(
		github_config: &GithubConfig,
		code: &str,
	) -> Result<Option<String>, reqwest::Error> {
		let client = reqwest::Client::new();
		let body: GithubToken = client
			.post("https://github.com/login/oauth/access_token")
			.header("Accept", "application/json")
			.header("User-Agent", GITHUB_USER_AGENT)
			.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
			.query(&[
				("client_id", github_config.client_id.as_str()),
				("client_secret", github_config.client_secret.as_str()),
				("code", code),
			])
			.send()
			.await?
			.json()
			.await?;

		// TODO handle Github's error message
		Ok(body.access_token)
	}

	/// Queries user informations from Github.
	///
	/// `access_token` is the access token.
	pub async fn query_info(access_token: &str) -> Result<GithubUser, reqwest::Error> {
		let client = reqwest::Client::new();
		client
			.get("https://api.github.com/user")
			.header("Accept", "application/json")
			.header("Authorization", format!("Bearer {}", access_token))
			.header("User-Agent", GITHUB_USER_AGENT)
			.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
			.send()
			.await?
			.json()
			.await
	}

	/// Returns the user with the given ID.
	///
	/// If the user doesn't exist, the function returns `None`.
	pub async fn from_id(db: &tokio_postgres::Client, id: &Oid) -> PgResult<Option<Self>> {
		Ok(db
			.query_opt("SELECT * FROM \"user\" WHERE id = $1", &[id])
			.await?
			.map(|r| FromRow::from_row(&r)))
	}

	/// Returns the user with the given Github ID.
	///
	/// `db` is the database.
	///
	/// If the user doesn't exist, the function returns `None`.
	pub async fn from_github_id(db: &tokio_postgres::Client, id: &i64) -> PgResult<Option<Self>> {
		db.query_opt("SELECT * FROM \"user\" WHERE github_id = $1", &[id])
			.await
			.map(|r| r.map(|r| FromRow::from_row(&r)))
	}

	/// Inserts the user in the database.
	///
	/// On success, the user's ID is updated.
	pub async fn insert(&mut self, db: &tokio_postgres::Client) -> PgResult<()> {
		let row = db
			.query_one(
				r#"INSERT INTO "user" (
			access_token,
			github_login,
			github_id,
			admin,
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		RETURNING id"#,
				&[
					&self.access_token,
					&self.github_login,
					&self.github_id,
					&self.admin,
				],
			)
			.await?;
		self.id = row.get("id");
		Ok(())
	}

	/// Returns the user of the current session.
	///
	/// `db` is the database.
	pub async fn current_user(
		db: &tokio_postgres::Client,
		session: &Session,
	) -> PgResult<Option<Self>> {
		let user_id = session.get::<Oid>("user_id").ok().flatten();
		match user_id {
			Some(user_id) => Self::from_id(db, &user_id).await,
			None => Ok(None),
		}
	}

	/// Checks the given session has admin permissions.
	pub async fn check_admin(db: &tokio_postgres::Client, session: &Session) -> PgResult<bool> {
		let user = Self::current_user(db, session).await?;
		Ok(user.map(|u| u.admin).unwrap_or(false))
	}
}
