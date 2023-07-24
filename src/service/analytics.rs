//! TODO doc

use crate::util;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// Each time a page is visited, an instance of this structure is saved.
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEntry {
	/// The date of the visit.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,

	/// The user's IP address. If unknown or removed, the value is `None`.
	pub peer_addr: Option<String>,
	/// The user agent. If unknown or removed, the value is `None`
	pub user_agent: Option<String>,

	/// The request method.
	pub method: String,
	/// The request URI.
	pub uri: String,

	/// If a user is logged, the name of this user.
	pub logged_user: Option<String>,
}

impl AnalyticsEntry {
	/// Inserts the analytics entry in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");
		// TODO do not count if IP is already present for the same path
		collection.insert_one(self, None).await.map(|_| ())
	}

	/// Aggregates entries.
	///
	/// `db` is the database.
	pub async fn aggregate(_db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		// TODO
		todo!();
	}
}
