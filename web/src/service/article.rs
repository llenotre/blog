//! This module handles articles.

use crate::util;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt;
use std::fmt::{Display, Formatter, Write};

/// Structure representing an article.
#[derive(Deserialize)]
pub struct Article {
	/// The article's title in the page's URL.
	pub url_title: String,
	/// The article's title.
	pub title: String,
	/// Timestamp at which the article has been posted.
	#[serde(with = "util::date_format")]
	pub post_date: DateTime<Utc>,
	/// The article's description.
	pub description: String,
	/// The URL to the cover image of the article.
	pub cover_url: String,
	/// The content of the article in markdown.
	pub content: String,
	/// The list of tags on the article.
	pub tags: Vec<String>,
	/// Tells whether the article is public.
	pub public: bool,
	/// Tells whether the article is reserved for sponsors.
	pub sponsor: bool,
	/// Tells whether comments are locked on the article.
	pub comments_locked: bool,
}

impl Article {
	/// Returns the URL of the article.
	pub fn get_url(&self) -> String {
		format!("https://blog.lenot.re/{}", self.url_title)
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

	/// Display the article as a sitemap element.
	pub fn display_sitemap(&self) -> ArticleSitemap {
		ArticleSitemap {
			article: self,
		}
	}

	/// Display the article as a RSS feed element.
	pub fn display_rss(&self) -> ArticleRss {
		ArticleRss {
			article: self,
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
			if self.article.public {
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
			r#"<a href="/{url_title}">
				<div class="article-element">
					<img class="article-cover" src="{cover_url}"></img>
					<div class="article-element-content">
						<h3>{title}</h3>
						<ul class="tags">
							<li><h6 style="color: gray;"><span id="date">{post_date}</span></h6></li>
							{tags}
						</ul>
						<p>
							{desc}
						</p>
					</div>
				</div>
			</a>"#,
			url_title = self.article.url_title,
			cover_url = self.article.cover_url,
			title = self.article.title,
			post_date = self.article.post_date.to_rfc3339(),
			tags = self.get_tags_html()?,
			desc = self.article.description,
		)
	}
}

pub struct ArticleSitemap<'a> {
	article: &'a Article,
}

impl<'a> Display for ArticleSitemap<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let url = self.article.get_url();
		let date = self.article.post_date.format("%Y-%m-%d");
		write!(
			f,
			"\t\t<url><loc>{url}</loc><lastmod>{date}</lastmod></url>"
		)
	}
}

pub struct ArticleRss<'a> {
	article: &'a Article,
}

impl<'a> Display for ArticleRss<'a> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"<item><guid>{url}</guid><title>{title}</title><link>{url}</link><pubDate>{post_date}</pubDate><description>{desc}</description></item>",
			url = self.article.get_url(),
			title = self.article.title,
			post_date = self.article.post_date.to_rfc2822(),
			desc = self.article.description
		)
	}
}
