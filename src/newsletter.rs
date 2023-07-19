//! This module implements newsletters.

use crate::util;
use crate::GlobalData;
use actix_web::error;
use actix_web::post;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Responder;
use bson::doc;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// An email address of a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct NewsletterEmail {
	/// The registered email. If `None`, the email has been anonymized.
	pub email: Option<String>,
	/// The date at which the user subscribed.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,
	/// Tells whether the user has unsubscribed.
	pub unsubscribed: bool,
}

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
	db.collection("newsletter_subscriber")
		.insert_one(
			NewsletterEmail {
				email: Some(info.email),
				date: Utc::now(),
				unsubscribed: false,
			},
			None,
		)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}

// TODO unsubscribe
