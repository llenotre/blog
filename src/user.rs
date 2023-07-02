//! This module implements user accounts.

use crate::GlobalData;
use actix_session::Session;
use actix_web::{error, get, http::StatusCode, web, web::Redirect, HttpResponseBuilder, Responder};
use bson::doc;
use bson::oid::ObjectId;
use serde::Deserialize;
use serde::Serialize;

/// The user agent for Github requests.
const GITHUB_USER_AGENT: &str = "maestro";
/// The Github API version.
const GITHUB_API_VERSION: &str = "2022-11-28";

// TODO update users from github data from time to time

/// Returns the authentication URL.
pub fn get_auth_url(client_id: &str) -> String {
	format!(
		"https://github.com/login/oauth/authorize?client_id={}",
		client_id
	)
}

/// Returns a redirection to the last article consulted by the session's user.
pub fn redirect_to_last_article(session: &Session) -> Redirect {
	let last_article = session.get::<String>("last_article");
	let uri = match last_article {
		Ok(Some(last_article)) => format!("/article/{}/", last_article),
		_ => "/".to_owned(),
	};

	Redirect::to(uri).see_other()
}

/// The query containing informations returned by Github for OAuth.
#[derive(Deserialize)]
pub struct OauthQuery {
	/// The code allowing to retrieve the user's token.
	code: Option<String>,
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
}

impl User {
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

	/// Checks the given session has admin permissions.
	///
	/// `db` is the database.
	pub async fn check_admin(
		db: &mongodb::Database,
		session: &Session,
	) -> Result<bool, mongodb::error::Error> {
		let user_id = session
			.get::<String>("user_id")
			.ok()
			.flatten()
			.and_then(|user_id| ObjectId::parse_str(user_id).ok());

		match user_id {
			Some(user_id) => {
				let user = Self::from_id(db, user_id).await?;
				Ok(user.map(|u| u.admin).unwrap_or(false))
			}

			None => Ok(false),
		}
	}
}

#[get("/auth")]
pub async fn auth(data: web::Data<GlobalData>) -> impl Responder {
	Redirect::to(get_auth_url(&data.client_id)).see_other()
}

#[get("/oauth")]
pub async fn oauth(
	data: web::Data<GlobalData>,
	query: web::Query<OauthQuery>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let Some(code) = query.into_inner().code else {
		return Err(error::ErrorBadRequest(""));
	};

	// Make call to Github to retrieve token
	let client = reqwest::Client::new();
	let body: GithubToken = client
		.post("https://github.com/login/oauth/access_token")
		.header("Accept", "application/json")
		.header("User-Agent", GITHUB_USER_AGENT)
		.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
		.query(&[
			("client_id", &data.client_id),
			("client_secret", &data.client_secret),
			("code", &code),
		])
		.send()
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?
		.json()
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let Some(access_token) = body.access_token else {
		return Err(error::ErrorInternalServerError(""));
	};

	// Get user ID
	let github_info = User::query_info(&access_token)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let db = data.get_database();
	let user = User::from_github_id(&db, github_info.id as _)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let user = match user {
		Some(user) => user,

		None => {
			// Insert new user
			let user = User {
				id: ObjectId::new(),

				access_token,
				github_info,

				admin: false,
				banned: false,
			};
			user.insert(&db)
				.await
				.map_err(|_| error::ErrorInternalServerError(""))?;

			user
		}
	};

	// Create user's session
	session.insert("user_id", user.id.to_hex())?;
	session.insert("user_login", user.github_info.login)?;

	// Redirect user
	Ok(redirect_to_last_article(&session))
}

#[get("/logout")]
pub async fn logout(session: Session) -> actix_web::Result<impl Responder> {
	// End session
	let _ = session.remove("user_id");
	let _ = session.remove("user_login");

	// Redirect user
	Ok(redirect_to_last_article(&session))
}

/// Avatar proxy, used to protect non-logged users from Github (RGPD)
#[get("/avatar/{user}")]
pub async fn avatar(user: web::Path<String>) -> actix_web::Result<impl Responder> {
	let user = user.into_inner();

	let client = reqwest::Client::new();
	let response = client
		.get(format!("https://github.com/{user}.png"))
		.send()
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let status = StatusCode::from_u16(response.status().as_u16()).unwrap();
	let mut builder = HttpResponseBuilder::new(status);
	if let Some(content_type) = response.headers().get("Content-Type") {
		Ok(builder
			.content_type(content_type)
			.insert_header(("Cache-Control", "max-age=604800"))
			.streaming(response.bytes_stream()))
	} else {
		Ok(builder
			.insert_header(("Cache-Control", "max-age=604800"))
			.streaming(response.bytes_stream()))
	}
}
