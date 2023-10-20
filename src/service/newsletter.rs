//! This module implements newsletters.

use crate::util::{now, PgResult};
use chrono::NaiveDateTime;

/// An email address of a newsletter subscriber.
pub struct NewsletterEmail<'s> {
	/// The registered email. If `None`, the email has been anonymized.
	pub email: Option<&'s str>,
	/// The date at which the user subscribed.
	pub subscribe_date: NaiveDateTime,
}

impl<'s> NewsletterEmail<'s> {
	/// Insert a new email in the newsletter subscribers list.
	pub async fn insert(db: &tokio_postgres::Client, email: &str) -> PgResult<()> {
		let now = now();
		db.execute(
			"INSERT INTO newsletter_subscriber (email, subscribe_date) VALUES ($1, $2)",
			&[&email, &now],
		)
		.await?;
		Ok(())
	}
}

// TODO unsubscribe
