use crate::newsletter::NewsletterEmail;
use crate::{util, GlobalData};
use actix_web::{error, post, web, HttpResponse, Responder};
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
		return Ok(HttpResponse::BadRequest().finish());
	}

	let db = data.get_database();
	NewsletterEmail::insert(&db, &info.email)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}
