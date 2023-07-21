use serde::{Deserialize, Serialize};
use actix_web::{error, HttpResponse, post, Responder, web};
use crate::{GlobalData, util};
use crate::newsletter::NewsletterEmail;

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
