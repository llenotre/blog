//! This module implements user accounts.

use crate::util;
use actix_session::Session;
use actix_web::web::Redirect;
use bson::doc;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
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

/// Returns a redirection to the last article consulted by the session's user.
pub fn redirect_to_last_article(session: &Session) -> Redirect {
	let last_article = session.get::<String>("last_article");
	let uri = match last_article {
		Ok(Some(last_article)) => format!("/article/{last_article}/redirect"),
		_ => "/".to_owned(),
	};

	Redirect::to(uri).see_other()
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
#[derive(Clone, Deserialize, Serialize)]
pub struct User {
	/// The user's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The user's Github access token.
	pub access_token: String,
	/// User informations.
	pub github_info: GithubUser,

	/// Tells whether the user is admin.
	pub admin: bool,
	/// Tells whether the user has been banned.
	pub banned: bool,

	/// The date/time at which the user registered.
	#[serde(with = "util::serde_date_time")]
	pub register_time: DateTime<Utc>,
	/// The date/time of the last post, used for cooldown.
	#[serde(with = "util::serde_date_time")]
	pub last_post: DateTime<Utc>,
}

impl User {
	/// Queries the access token from the given `code` returned by Github.
	pub async fn query_access_token(client_id: &str, client_secret: &str, code: &str) -> Result<Option<String>, reqwest::Error> {
		let client = reqwest::Client::new();
		let body: GithubToken = client
			.post("https://github.com/login/oauth/access_token")
			.header("Accept", "application/json")
			.header("User-Agent", GITHUB_USER_AGENT)
			.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
			.query(&[
				("client_id", client_id),
				("client_secret", client_secret),
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
	/// `db` is the database.
	///
	/// If the user doesn't exist, the function returns `None`.
	pub async fn from_id(
		db: &mongodb::Database,
		id: ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("user");
		collection.find_one(doc! {"_id": id}, None).await
	}

	/// Returns the user with the given Github ID.
	///
	/// `db` is the database.
	///
	/// If the user doesn't exist, the function returns `None`.
	pub async fn from_github_id(
		db: &mongodb::Database,
		id: u64,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("user");
		collection
			.find_one(doc! {"github_info.id": id as i64}, None)
			.await
	}

	/// Inserts or updates the user in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("user");
		collection.insert_one(self, None).await.map(|_| ())
	}

	/// Updates the user's cooldown.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `last_post` is the date/time of the last post from the user.
	pub async fn update_cooldown(
		&self,
		db: &mongodb::Database,
		last_post: DateTime<Utc>,
	) -> Result<(), mongodb::error::Error> {
		let last_post = last_post.to_rfc3339();

		let collection = db.collection::<Self>("user");
		collection
			.update_one(
				doc! {
					"_id": self.id,
				},
				doc! {
					"$set": doc! {
						"last_post": last_post,
					}
				},
				None,
			)
			.await
			.map(|_| ())
	}

	/// Returns the user of the current session.
	///
	/// `db` is the database.
	pub async fn current_user(
		db: &mongodb::Database,
		session: &Session,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let user_id = session
			.get::<String>("user_id")
			.ok()
			.flatten()
			.and_then(|user_id| ObjectId::parse_str(user_id).ok());
		match user_id {
			Some(user_id) => Self::from_id(db, user_id).await,
			None => Ok(None),
		}
	}

	/// Checks the given session has admin permissions.
	///
	/// `db` is the database.
	pub async fn check_admin(
		db: &mongodb::Database,
		session: &Session,
	) -> Result<bool, mongodb::error::Error> {
		let user = Self::current_user(db, session).await?;
		Ok(user.map(|u| u.admin).unwrap_or(false))
	}
}
