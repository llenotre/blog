//! TODO doc

use actix_web::{
	HttpResponse,
	Responder,
	get,
	web,
};
use crate::GlobalData;
use serde::Deserialize;

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

	// TODO insert user

	// TODO redirect user to the page before login
	HttpResponse::Ok().finish()
}
