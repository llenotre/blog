//! TODO doc

use crate::util;
use crate::GlobalData;
use actix_web::error;
use actix_web::post;
use actix_web::web;
use actix_web::HttpResponse;
use actix_web::Responder;
use bson::doc;
use bson::oid::ObjectId;
use bson::Bson;
use chrono::DateTime;
use chrono::Utc;
use futures_util::TryStreamExt;
use lettre::Message;
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

/// The state of a message.
#[derive(Deserialize, Serialize)]
pub enum NewsletterMessageState {
	/// The message is waiting to be sent.
	Pending,
	/// Message could not be delivered.
	Failed { reason: String },
	/// The message has been sent successfuly.
	Sent {
		/// The date at which the message has been sent.
		#[serde(with = "util::serde_date_time")]
		date: DateTime<Utc>,
	},
	/// The email has been cancelled.
	Cancelled,
}

/// An email message either sent or to be sent.
#[derive(Deserialize, Serialize)]
pub struct NewsletterMessage {
	/// The ID of the template associated with the message.
	pub template_id: ObjectId,

	/// Destination email. If `None`, the email has been anonymized.
	pub to: Option<String>,
	/// The email's content.
	pub content: Vec<u8>,

	/// The state of the message.
	pub state: NewsletterMessageState,
}

/// An email template to be sent.
#[derive(Deserialize, Serialize)]
pub struct NewsletterTemplate {
	/// The template's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The name of the template.
	pub name: String,
	/// The number of emails to send per second.
	pub send_speed: u32,

	/// The email's title.
	pub title: String,
	/// The content template.
	pub content_template: String,
}

/// Component scheduling email sending.
pub struct EmailWorker {
	/// Global data, containing database client.
	data: web::Data<GlobalData>,
}

impl EmailWorker {
	/// Creates a new instance.
	pub fn new(data: web::Data<GlobalData>) -> Self {
		Self {
			data,
		}
	}

	/// Runs the worker.
	pub async fn run(&self) {
		// TODO
	}

	/// Sends a mail.
	pub async fn enqueue(&self, msg: NewsletterMessage) -> Result<(), mongodb::error::Error> {
		let coll = self.data.get_database().collection("newsletter_message");
		coll.insert_one(msg, None).await.map(|_| ())
	}

	/// Launches an email campain on all registered emails.
	pub async fn launch_campain(
		&self,
		template: &NewsletterTemplate,
	) -> Result<(), mongodb::error::Error> {
		let subscribers_coll = self
			.data
			.get_database()
			.collection::<NewsletterEmail>("newsletter_subscriber");
		let mut stream = subscribers_coll
			.find(
				doc! {
					"email": {"$ne": Bson::Null },
					"unsubscribed": false
				},
				None,
			)
			.await?;

		// TODO implement errors handling, retrying on reboot, etc...
		while let Some(email) = stream.try_next().await? {
			let Some(email) = email.email else {
                continue;
            };

			// Build mail
			let message = Message::builder()
				.from("newsletter <newsletter@blog.lenot.re>".parse().unwrap())
				.to(email.parse().unwrap())
				.subject(template.title.clone())
				// TODO fill template with unsubscribe URL
				.body(template.content_template.clone())
				.unwrap();
			// TODO sign message (DKIM)

			// Enqueue mail
			self.enqueue(NewsletterMessage {
				template_id: ObjectId::new(),

				to: Some(email),
				content: message.formatted(),

				state: NewsletterMessageState::Pending,
			})
			.await?;
		}

		Ok(())
	}
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
				date: chrono::offset::Utc::now(),
				unsubscribed: false,
			},
			None,
		)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}

// TODO unsubscribe
