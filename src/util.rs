//! Module implementing utilities.

use lazy_static::lazy_static;
use regex::Regex;

/// Module handling serialization/deserialization of dates.
pub mod serde_date_time {
	use chrono::DateTime;
	use chrono::TimeZone;
	use chrono::Utc;
	use serde::Deserialize;
	use serde::Deserializer;
	use serde::Serializer;

	/// Serialization format.
	pub const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

	/// Serialize
	pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let s = format!("{}", date.format(FORMAT));
		serializer.serialize_str(&s)
	}

	/// Deserialize
	pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Utc.datetime_from_str(&s, FORMAT)
			.map_err(serde::de::Error::custom)
	}
}

lazy_static! {
	/// Email validation regex.
	static ref EMAIL_VALIDATION: Regex = Regex::new(r##"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$"##).unwrap();
}

/// Tells whether the given email is valid.
pub fn validate_email(email: &str) -> bool {
	EMAIL_VALIDATION.is_match(email)
}
