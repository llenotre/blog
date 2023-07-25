//! TODO doc

use bson::doc;
use crate::util;
use chrono::{DateTime, Duration};
use chrono::Utc;
use futures_util::{StreamExt};
use serde::Deserialize;
use serde::Serialize;

/// Informations about the user. This is an enumeration because client data has to be aggregated on a regular basis for GDPR reasons.
#[derive(Deserialize, Serialize)]
pub enum UserInfo {
	/// Sensitive data, not aggregated yet.
	Sensitive {
		/// The user's IP address. If unknown or removed, the value is `None`.
		peer_addr: Option<String>,
		/// The user agent. If unknown or removed, the value is `None`
		user_agent: Option<String>,
	},
	/// Aggregated data.
	Aggregated {
		// TODO geolocation
		// TODO client's device specificities
	}
}

/// Each time a page is visited, an instance of this structure is saved.
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEntry {
	/// The date of the visit.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,

	/// User's info.
	#[serde(flatten)]
	pub user_info: UserInfo,

	/// The request method.
	pub method: String,
	/// The request URI.
	pub uri: String,
}

impl AnalyticsEntry {
	/// Inserts the analytics entry in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");

		let peer_addr = match &self.user_info {
			UserInfo::Sensitive { peer_addr, .. } => peer_addr.as_deref(),
			_ => None,
		};
		let entry = collection.find_one(doc!{
			"peer_addr": peer_addr,
			"uri": &self.uri,
		}, None).await?;
		// Do not count the same client twice
		if entry.is_none() {
			collection.insert_one(self, None).await?;
		}

		Ok(())
	}

	/// Aggregates entries.
	///
	/// `db` is the database.
	pub async fn aggregate(db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");

		let oldest = Utc::now() - Duration::hours(24);
		// Get the list of entries to aggregate
		let mut entries_iter = collection.find(doc!{
			"date": { "$lt": oldest },
		}, None)
			.await?;
		while let Some(mut e) = entries_iter.next().await.transpose()? {
			let UserInfo::Sensitive {
				peer_addr,
				user_agent,
			} = e.user_info else {
				continue;
			};

			// Get geolocation from peer address
			// TODO

			// Parse user agent
			// TODO

			e.user_info = UserInfo::Aggregated {
			};
			// TODO update entry
		}

		Ok(())
	}
}
