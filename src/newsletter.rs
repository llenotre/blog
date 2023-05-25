//! TODO doc

use serde::Deserialize;
use serde::Serialize;

/// An email address of a newsletter subscriber.
#[derive(Deserialize, Serialize)]
pub struct NewsletterEmail {
	pub email: String,
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
