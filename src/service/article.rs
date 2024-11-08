//! This module handles articles.

use crate::util;
use crate::util::now;
use anyhow::bail;
use anyhow::Result;
use chrono::{DateTime, Utc};
use lol_html::{element, HtmlRewriter};
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
use std::fmt::{Display, Formatter, Write};
use std::fs::DirEntry;
use std::path::Path;
use std::{fmt, fs, io};
use tracing::info;

/// An article.
#[derive(Deserialize)]
pub struct Article {
	/// The article's slug.
	#[serde(default)]
	pub slug: String,
	/// The article's title.
	pub title: String,
	/// Timestamp at which the article has been posted.
	#[serde(with = "util::date_format")]
	pub post_date: DateTime<Utc>,
	/// The article's description.
	pub description: String,
	/// The URL to the cover image of the article.
	pub cover_url: String,
	/// The list of tags on the article.
	#[serde(default)]
	pub tags: Vec<String>,
}

impl Article {
	/// Compiles all articles and returns them along with the resulting HTML, sorted by decreasing
	/// post date.
	pub fn compile_all(articles_path: &Path) -> Result<Vec<(Article, String)>> {
		let filter = |e: io::Result<DirEntry>| {
			let e = e?;
			if e.file_type()?.is_dir() && e.file_name() != ".git" {
				Ok(Some(e))
			} else {
				Ok(None)
			}
		};
		let articles: Result<Vec<(Self, String)>> = fs::read_dir(articles_path)?
			.filter_map(|e| filter(e).transpose())
			.map(|e: io::Result<DirEntry>| {
				let e = e?;
				// Read metadata
				let manifest_path = e.path().join("manifest.toml");
				let manifest = fs::read_to_string(manifest_path)?;
				let mut manifest: Self = match toml::from_str(&manifest) {
					Ok(m) => m,
					Err(err) => bail!(
						"failed to read article {name}: {err}",
						name = e.file_name().to_string_lossy()
					),
				};
				if manifest.slug.is_empty() {
					manifest.slug = e.file_name().to_string_lossy().into_owned();
				}

				// Read and compile content
				let content_path = e.path().join("content.md");
				let content = fs::read_to_string(content_path)?;
				let content = compile_content(&content);
				info!(
					title = manifest.title,
					public = manifest.is_public(),
					"compiled article"
				);

				Ok((manifest, content))
			})
			.collect();
		let mut articles = articles?;
		articles.sort_unstable_by(|(a1, _), (a2, _)| a1.post_date.cmp(&a2.post_date).reverse());
		Ok(articles)
	}

	/// Returns the path to the article.
	pub fn get_path(&self) -> String {
		format!("/a/{}", self.slug)
	}

	/// Returns the URL of the article.
	pub fn get_url(&self) -> String {
		format!("https://blog.lenot.re/a/{}", self.slug)
	}

	/// Tells whether the article is public.
	pub fn is_public(&self) -> bool {
		self.post_date <= now().and_utc()
	}
}

/// Display an article as an element on the index page.
pub struct ArticleListHtml<'a>(pub &'a Article);

impl ArticleListHtml<'_> {
	/// Returns the HTML representing the article's tags.
	fn get_tags_html(&self) -> Result<String, fmt::Error> {
		let mut html = String::new();
		self.0
			.tags
			.iter()
			.try_for_each(|tag| write!(html, r#"<li class="tag">{tag}</li>"#))?;
		Ok(html)
	}
}

impl Display for ArticleListHtml<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			r#"<a href="{path}">
				<div class="article-element">
					<img class="article-cover" src="{cover_url}" alt="{title}"></img>
					<div class="article-element-content">
						<h3>{title}</h3>
						<ul class="tags">
							<li class="date"><span id="date">{post_date}</span></li>
							{tags}
						</ul>
						<p>
							{desc}
						</p>
					</div>
				</div>
			</a>"#,
			path = self.0.get_path(),
			cover_url = self.0.cover_url,
			title = self.0.title,
			post_date = self.0.post_date.to_rfc3339(),
			tags = self.get_tags_html()?,
			desc = self.0.description,
		)
	}
}

/// Display an article as a sitemap element.
pub struct ArticleSitemap<'a>(pub &'a Article);

impl Display for ArticleSitemap<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let url = self.0.get_url();
		let date = self.0.post_date.format("%Y-%m-%d");
		write!(
			f,
			"\n\t<url><loc>{url}</loc><lastmod>{date}</lastmod></url>"
		)
	}
}

/// Display an article as an RSS element.
pub struct ArticleRss<'a>(pub &'a Article);

impl Display for ArticleRss<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"<item><guid>{url}</guid><title>{title}</title><link>{url}</link><pubDate>{post_date}</pubDate><description>{desc}</description></item>",
			url = self.0.get_url(),
			title = self.0.title,
			post_date = self.0.post_date.to_rfc2822(),
			desc = self.0.description
		)
	}
}

/// Compiles the given content from Markdown into HTML.
fn compile_content(content: &str) -> String {
	// Compile to HTML
	let parser = Parser::new_ext(&content, Options::all());
	let mut content = String::new();
	html::push_html(&mut content, parser);

	// Rewrite HTML
	let mut output = vec![];
	let mut rewriter = HtmlRewriter::new(
		lol_html::Settings {
			element_content_handlers: vec![
				// TODO article summary
				// TODO enlarge image when clicking on it
				// Lazy loading assets
				element!("img,video", |e| {
					e.set_attribute("loading", "lazy").unwrap();
					Ok(())
				}),
				// Add target="_blank" to links that require it
				element!("a[href]", |e| {
					let href = e.get_attribute("href").unwrap();
					if let Some(href) = href.strip_prefix("_") {
						e.set_attribute("href", href).unwrap();
						e.set_attribute("target", "_blank").unwrap();
					}
					Ok(())
				}),
			],
			..lol_html::Settings::default()
		},
		|c: &[u8]| output.extend_from_slice(c),
	);
	rewriter.write(content.as_bytes()).unwrap();
	rewriter.end().unwrap();

	String::from_utf8(output).unwrap()
}
