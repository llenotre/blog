use crate::service::user;
use crate::service::user::User;
use crate::util::now;
use crate::GlobalData;
use actix_session::Session;
use actix_web::http::StatusCode;
use actix_web::web::Redirect;
use actix_web::{error, get, web, HttpResponseBuilder, Responder};
use serde::Deserialize;

#[get("/auth")]
pub async fn auth(data: web::Data<GlobalData>) -> impl Responder {
	Redirect::to(user::get_auth_url(&data.client_id)).see_other()
}

/// The query containing informations returned by Github for OAuth.
#[derive(Deserialize)]
pub struct OauthQuery {
	/// The code allowing to retrieve the user's token.
	code: Option<String>,
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

	// Get access token
	let access_token = User::query_access_token(&data.client_id, &data.client_secret, &code)
		.await
		.map_err(|error| {
			tracing::error!(error = %error, "could not retrieve access token from Github");
			error::ErrorInternalServerError("")
		})?;
	let Some(access_token) = access_token else {
		// TODO log
		return Err(error::ErrorInternalServerError(""));
	};

	// Get user ID
	let github_user = User::query_info(&access_token).await.map_err(|error| {
		tracing::error!(error = %error, "could not retrieve user's informations from Github");
		error::ErrorInternalServerError("")
	})?;

	let user = User::from_github_id(&data.db, &(github_user.id as _))
		.await
		.map_err(|error| {
			tracing::error!(error = %error, "could not reach database");
			error::ErrorInternalServerError("")
		})?;
	let user = match user {
		Some(user) => user,

		None => {
			// Insert new user
			let user = User {
				id: 0,

				access_token,
				github_id: github_user.id,
				github_login: github_user.login,
				github_html_url: github_user.html_url,

				admin: false,
				banned: false,

				register_date: now(),
				last_post: Default::default(),
			};
			user.insert(&data.db).await.map_err(|error| {
				tracing::error!(error = %error, "could not reach database");
				error::ErrorInternalServerError("")
			})?;
			user
		}
	};

	user::create_session(&session, &user)?;
	Ok(user::redirect_to_last_article(&session))
}

#[get("/logout")]
pub async fn logout(session: Session) -> actix_web::Result<impl Responder> {
	let redirect = user::redirect_to_last_article(&session);
	session.purge();
	Ok(redirect)
}

/// Avatar proxy, used to protect non-logged users from Github (GDPR)
#[get("/avatar/{user}")]
pub async fn avatar(user: web::Path<String>) -> actix_web::Result<impl Responder> {
	let user = user.into_inner();

	let client = reqwest::Client::new();
	let response = client
		.get(format!("https://github.com/{user}.png"))
		.send()
		.await
		.map_err(|error| {
			tracing::error!(error = %error, user, "could not get avatar from Github");
			error::ErrorInternalServerError("")
		})?;

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
