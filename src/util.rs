//! Module implementing utilities.

use chrono::DateTime;
use chrono::Utc;
use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;

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
		DateTime::parse_from_rfc3339(&s)
			.map(|d| d.with_timezone(&Utc))
			.map_err(serde::de::Error::custom)
	}
}

/// Wrapper used to allow serializing/deserializing `Option<DateTime<T>>`.
#[derive(Debug, Deserialize, Serialize)]
pub struct DateTimeWrapper(#[serde(with = "serde_date_time")] pub DateTime<Utc>);

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
