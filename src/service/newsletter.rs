//! This module implements newsletters.

use crate::util;
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
	pub subscribe_date: DateTime<Utc>,
}

impl<'s> NewsletterEmail<'s> {
	/// Insert a new email in the newsletter subscribers list.
	pub async fn insert(db: &mongodb::Database, email: &str) -> Result<(), mongodb::error::Error> {
		let collection = db.collection("newsletter_subscriber");
		collection
			.insert_one(
				NewsletterEmail {
					email: Some(email),
					subscribe_date: Utc::now(),
				},
				None,
			)
			.await
			.map(|_| ())
	}
}

// TODO unsubscribe
