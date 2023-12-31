//! Module implementing utilities.

use chrono::{NaiveDateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;
use tokio_postgres::Row;

/// Result with PostgreSQL error.
pub type PgResult<T> = Result<T, tokio_postgres::Error>;
/// Database primary key.
pub type Oid = i32;

/// An object that can be instanciated from a SQL row.
pub trait FromRow {
	/// Creates an object from the given SQL row.
	///
	/// If the given row is invalid, the function panics.
	fn from_row(row: &Row) -> Self
	where
		Self: Sized;
}

/// Returns the current date time on the UTC timezone.
pub fn now() -> NaiveDateTime {
	Utc::now().naive_utc()
}

/// Tells whether the given email is valid.
pub fn validate_email(email: &str) -> bool {
	static EMAIL_VALIDATION: OnceLock<Regex> = OnceLock::new();
	let regex = EMAIL_VALIDATION.get_or_init(|| {
		Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$").unwrap()
	});
	regex.is_match(email)
}

/// Date deserialization.
pub mod date_format {
	use chrono::{DateTime, NaiveDateTime, Utc};
	use serde::{self, Deserialize, Deserializer};

	const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

	pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
		Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
	}
}
