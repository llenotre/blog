//! Module implementing utilities.

use base64::engine::general_purpose;
use base64::Engine;
use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;

/// TODO doc
pub type PgResult<T> = Result<T, tokio_postgres::Error>;

/// TODO doc
pub trait FromRow {
	/// TODO doc
	fn from_row(row: &Row) -> Self;
}

lazy_static! {
	/// Email validation regex.
	static ref EMAIL_VALIDATION: Regex = Regex::new(r##"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$"##).unwrap();
}

/// Tells whether the given email is valid.
pub fn validate_email(email: &str) -> bool {
	EMAIL_VALIDATION.is_match(email)
}

/// Encodes an ID.
pub fn encode_id(id: &ObjectId) -> String {
	general_purpose::URL_SAFE_NO_PAD.encode(id.bytes())
}

/// Decodes an ID.
///
/// If the given ID is invalid, the function returns None.
pub fn decode_id(id: &str) -> Option<ObjectId> {
	general_purpose::URL_SAFE_NO_PAD
		.decode(id)
		.ok()
		.and_then(|id| {
			let id: [u8; 12] = id.as_slice().try_into().ok()?;
			Some(id)
		})
		.map(|id| ObjectId::from(id))
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
