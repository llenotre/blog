//! This module implements newsletters.

use crate::util;
use bson::doc;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// An email address of a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct NewsletterEmail<'s> {
	/// The registered email. If `None`, the email has been anonymized.
	pub email: Option<&'s str>,
	/// The date at which the user subscribed.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,
	/// Tells whether the user has unsubscribed.
	pub unsubscribed: bool,
}

impl<'s> NewsletterEmail<'s> {
	/// Insert a new email in the newsletter subscribers list.
	pub async fn insert(db: &mongodb::Database, email: &str) -> Result<(), mongodb::error::Error> {
		let collection = db.collection("newsletter_subscriber");
		collection
			.insert_one(
				NewsletterEmail {
					email: Some(email),
					date: Utc::now(),
					unsubscribed: false,
				},
				None,
			)
			.await
			.map(|_| ())
	}
}

// TODO unsubscribe
