//! This module implements analytics.

use chrono::DateTime;
use chrono::Utc;
use crate::util;
use serde::Deserialize;
use serde::Serialize;

/// Each time a page is visited, an instance of this structure is saved.
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEntry {
	/// The date of the visit.
	#[serde(with = "util::serde_date_time")]
	date: DateTime<Utc>,

	/// The user's address.
	address: String,
	/// The user agent.
	user_agent: Option<String>,

	/// If a user is logged, the name of this user.
	logged_user: Option<String>,
}
