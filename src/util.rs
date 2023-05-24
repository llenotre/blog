//! Module implementing utilities.

/// Ceil division.
pub fn ceil_div(a: u32, b: u32) -> u32 {
	if a % b != 0 {
		a / b + 1
	} else {
		a / b
	}
}

/// Module handling serialization/deserialization of dates.
pub mod serde_date_time {
	use chrono::DateTime;
	use chrono::TimeZone;
	use chrono::Utc;
	use serde::Deserialize;
	use serde::Deserializer;
	use serde::Serializer;

	/// Serialization format.
	const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

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
