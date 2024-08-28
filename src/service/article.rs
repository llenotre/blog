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
use std::{fmt, fs, io};
use tracing::info;

/// The path to the articles' sources.
const ARTICLES_PATH: &str = "articles/";

/// Structure representing an article.
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
	/// Tells whether the article is public.
	#[serde(default)]
	pub public: bool,
	/// Tells whether the article is reserved for sponsors.
	#[serde(default)]
	pub sponsor: bool,
}

impl Article {
	/// Compiles all articles and returns them along with the resulting HTML, sorted by decreasing
	/// post date.
	pub fn compile_all() -> Result<Vec<(Article, String)>> {
		let filter = |e: io::Result<DirEntry>| {
			let e = e?;
			if e.file_type()?.is_dir() && e.file_name() != ".git" {
				Ok(Some(e))
			} else {
				Ok(None)
			}
		};
		let articles: Result<Vec<(Self, String)>> = fs::read_dir(ARTICLES_PATH)?
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
		self.public && self.post_date <= now().and_utc()
	}

	/// Display the article in the list on the main page.
	///
	/// `admin` tells whether the user is admin.
	pub fn display_list_html(&self, admin: bool) -> ArticleListHtml {
		ArticleListHtml {
			article: self,
			admin,
		}
	}
}

pub struct ArticleListHtml<'a> {
	article: &'a Article,
	/// Tells whether the client is logged as an admin.
	admin: bool,
}

impl<'a> ArticleListHtml<'a> {
	/// Returns the HTML representing the article's tags.
	fn get_tags_html(&self) -> Result<String, fmt::Error> {
		let mut html = String::new();
		if self.admin {
			if self.article.is_public() {
				write!(html, r#"<li class="tag">Public</li>"#)?;
			} else {
				write!(html, r#"<li class="tag">Private</li>"#)?;
			}
		}
		if self.article.sponsor {
			write!(
				html,
				r#"<li class="tag"><i>Sponsors early access</i>&nbsp;&nbsp;&nbsp;❤️</li>"#
			)?;
		}
		self.article
			.tags
			.iter()
			.try_for_each(|tag| write!(html, r#"<li class="tag">{tag}</li>"#))?;
		Ok(html)
	}
}

impl<'a> Display for ArticleListHtml<'a> {
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
			path = self.article.get_path(),
			cover_url = self.article.cover_url,
			title = self.article.title,
			post_date = self.article.post_date.to_rfc3339(),
			tags = self.get_tags_html()?,
			desc = self.article.description,
		)
	}
}

/// Display an article as a sitemap element.
pub struct ArticleSitemap<'a>(pub &'a Article);

impl<'a> Display for ArticleSitemap<'a> {
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

impl<'a> Display for ArticleRss<'a> {
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
