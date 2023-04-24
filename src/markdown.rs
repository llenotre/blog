//! This module handles Markdown.

use pulldown_cmark::{html, Options, Parser};

/// Converts the given Markdown to HTML.
pub fn to_html(md: &str) -> String {
    let options = Options::all();
    let parser = Parser::new_ext(md, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}
