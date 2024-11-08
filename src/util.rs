//! Module implementing utilities.

use chrono::{NaiveDateTime, Utc};

/// Returns the current date time on the UTC timezone.
pub fn now() -> NaiveDateTime {
	Utc::now().naive_utc()
}

/// Date deserialization.
pub mod date_format {
	use chrono::{DateTime, NaiveDateTime, Utc};
	use serde::{Deserialize, Deserializer};

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
