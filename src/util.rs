//! Module implementing utilities.

use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use tokio_postgres::Row;

/// Result with PostgreSQL error.
pub type PgResult<T> = Result<T, tokio_postgres::Error>;
/// Database primary key.
pub type Oid = u32;

/// An object that can be instanciated from a SQL row.
pub trait FromRow {
	/// Creates an object from the given SQL row.
	///
	/// If the given row is invalid, the function panics.
	fn from_row(row: &Row) -> Self
	where
		Self: Sized;
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
