//! Module implementing utilities.

use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;

/// Module handling serialization/deserialization of dates.
pub mod serde_date_time {
	use chrono::DateTime;
	use chrono::Utc;
	use serde::Deserialize;
	use serde::Deserializer;
	use serde::Serializer;

	pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&date.to_rfc3339())
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let r = DateTime::parse_from_rfc3339(&s)
			.map(|d| d.with_timezone(&Utc))
			.map_err(serde::de::Error::custom);
		dbg!(&r);
		r
	}
}

/// Module handling serialization/deserialization of options of dates.
pub mod serde_option_date_time {
	use chrono::DateTime;
	use chrono::Utc;
	use serde::de::IntoDeserializer;
	use serde::Deserialize;
	use serde::Deserializer;
	use serde::Serializer;

	pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match date {
			Some(date) => serializer.serialize_some(&date.to_rfc3339()),
			None => serializer.serialize_none(),
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<&str>::deserialize(deserializer)?
			.map(|s| {
				let deserializer = s.into_deserializer();
				super::serde_date_time::deserialize(deserializer)
			})
			.transpose()
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

/// Converts the given Markdown to HTML.
///
/// Arguments:
/// - `md` is the Markdown content.
/// - `escape` tells whether unsafe HTML must be sanitized.
pub fn markdown_to_html(md: &str, escape: bool) -> String {
	let options = Options::all();
	let parser = Parser::new_ext(md, options);

	let mut html_output = String::new();
	html::push_html(&mut html_output, parser);

	if escape {
		ammonia::clean(&html_output)
	} else {
		html_output
	}
}
