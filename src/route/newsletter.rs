use crate::service::newsletter::NewsletterEmail;
use crate::{util, GlobalData};
use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

/// Payload of request to register a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct SubscribePayload {
	/// The email of the subscriber.
	email: String,
}

#[post("/newsletter/subscribe")]
pub async fn subscribe(
	data: web::Data<GlobalData>,
	info: web::Json<SubscribePayload>,
) -> actix_web::Result<impl Responder> {
	let info = info.into_inner();
	if !util::validate_email(&info.email) {
		return Ok(HttpResponse::BadRequest()
			.content_type("text/plain")
			.body("invalid email address"));
	}

	let db = data.get_database();
	if NewsletterEmail::insert(&db, &info.email).await.is_err() {
		return Ok(HttpResponse::InternalServerError()
			.content_type("text/plain")
			.body("internal server error"));
	}

	Ok(HttpResponse::Ok().finish())
}
