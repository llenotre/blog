//! This module handles Markdown.

use pulldown_cmark::{html, Options, Parser};

/// Converts the given Markdown to HTML.
///
/// Arguments:
/// - `md` is the Markdown content.
/// - `escape` tells whether unsafe HTML must be sanitized.
pub fn to_html(md: &str, escape: bool) -> String {
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
