//! TODO doc

use actix_web::{
	HttpResponse,
	Responder,
	get,
	web,
};
use crate::GlobalData;
use serde::Deserialize;
use serde::Serialize;

/// The Github API version.
const GITHUB_API_VERSION: &str = "2022-11-28";

/// Returns the authentication URL.
pub fn get_auth_url(global: &GlobalData) -> String {
	format!(
		"https://github.com/login/oauth/authorize?client_id={}",
		global.client_id
	)
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
#[derive(Deserialize, Serialize)]
pub struct GithubUser {
	/// The user's login.
	login: String,
	/// The user's ID.
	id: u64,
	/// The URL to the user's avatar.
	avatar_url: String,
	/// The URL to the user's profile.
	html_url: String,
}

/// A user, who can post comments, or if admin, administrate the website.
#[derive(Deserialize, Serialize)]
pub struct User {
	/// User informations.
	github_info: GithubUser,

	/// Tells whether the user is admin.
	admin: bool,
	/// If the user is banned, the reason for the user being banned.
	ban_reason: Option<String>,
}

impl User {
	/// Query current user informations .
	///
	/// `access_token` is the access token.
	pub async fn query_info(access_token: &str) -> Result<GithubUser, reqwest::Error> {
		let client = reqwest::Client::new();
		client.post("https://api.github.com/user")
			.header("Accept", "application/json")
			.header("Authorization", format!("Bearer {}", access_token))
			.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
			.send()
			.await?
			.json()
			.await
	}
}

#[get("/oauth")]
pub async fn oauth(
	data: web::Data<GlobalData>,
	query: web::Query<OauthQuery>
) -> impl Responder {
	let Some(code) = query.into_inner().code else {
		// TODO error?
		return HttpResponse::Ok().finish();
	};

	// Make call to Github to retrieve token
	let url = format!(
		"https://github.com/login/oauth/access_token?client_id={}&client_secret={}&code={}",
		data.client_id,
		data.client_secret,
		code
	);
	let client = reqwest::Client::new();
	let body: GithubToken = client.post(url)
		.header("Accept", "application/json")
		.header("X-GitHub-Api-Version", GITHUB_API_VERSION)
		.send()
		.await
		.unwrap() // TODO handle error
		.json()
		.await
		.unwrap(); // TODO handle error

	let Some(access_token) = body.access_token else {
		// TODO error
		return HttpResponse::Ok().finish();
	};

	// Get user informations
	let info = User::query_info(&access_token).await.unwrap(); // TODO handle error

	// Insert or update user
	// TODO

	// TODO redirect user to the page before login
	HttpResponse::Ok().finish()
}
