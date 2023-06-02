//! TODO doc

use actix_web::HttpResponse;
use actix_web::Responder;
use actix_web::error;
use actix_web::post;
use actix_web::web;
use bson::doc;
use chrono::DateTime;
use chrono::Utc;
use crate::GlobalData;
use crate::util;
use mongodb::options::ReplaceOptions;
use serde::Deserialize;
use serde::Serialize;

/// An email address of a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct NewsletterEmail {
	pub email: String,
	/// The date at which the user subscribed.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,
}

/// The state of a message.
#[derive(Deserialize, Serialize)]
pub enum NewsletterMessageState {
	/// The message is waiting to be sent.
	Pending,
	/// Message could not be delivered.
	Failed,
	/// The message has been sent successfuly.
	Sent {
		/// The date at which the message has been sent.
		#[serde(with = "util::serde_date_time")]
		date: DateTime<Utc>
	},
}

/// An email message either sent or to be sent.
#[derive(Deserialize, Serialize)]
pub struct NewsletterMessage {
	/// Source email.
	pub from: String,
	/// Destination email.
	pub to: String,
	/// The email's content.
	pub content: String,

	/// The state of the message.
	pub state: NewsletterMessageState,
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
		.replace_one(
			doc! {
				"email": &info.email,
			},
			NewsletterEmail {
				email: info.email,
				date: chrono::offset::Utc::now(),
			},
			Some(ReplaceOptions::builder().upsert(true).build())
		)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}

// TODO unsubscribe
