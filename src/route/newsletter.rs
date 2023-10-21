use crate::service::newsletter::NewsletterEmail;
use crate::{util, GlobalData};
use actix_web::http::header::ContentType;
use actix_web::{error, get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::error;

/// Payload of request to register a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct SubscribePayload {
	/// The email of the subscriber.
	email: String,
}

/// Payload of request to unregister a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct UnsubscribePayload {
	/// The unsubscribe token.
	token: String,
}

#[post("/newsletter/subscribe")]
pub async fn subscribe(
	data: web::Data<GlobalData>,
	info: web::Json<SubscribePayload>,
) -> actix_web::Result<impl Responder> {
	let info = info.into_inner();
	// Validate payload
	if !util::validate_email(&info.email) {
		return Ok(HttpResponse::BadRequest()
			.content_type("text/plain")
			.body("invalid email address"));
	}
	// Insert in DB
	if NewsletterEmail::insert(&data.db, &info.email)
		.await
		.is_err()
	{
		return Ok(HttpResponse::InternalServerError()
			.content_type("text/plain")
			.body("internal server error"));
	}
	Ok(HttpResponse::Ok().finish())
}

#[get("/newsletter/unsubscribe")]
pub async fn unsubscribe(
	data: web::Data<GlobalData>,
	info: web::Json<UnsubscribePayload>,
) -> actix_web::Result<impl Responder> {
	let success = NewsletterEmail::unsubscribe_from_token(&data.db, &info.token)
		.await
		.map_err(|e| {
			error!(error = %e, "postgres: cannot unsubscribe");
			error::ErrorInternalServerError("")
		})?;
	if success {
		let html = include_str!("../../pages/newsletter_unsubscribe.html");
		Ok(HttpResponse::Ok()
			.content_type(ContentType::html())
			.body(html))
	} else {
		Err(error::ErrorNotFound(""))
	}
}
