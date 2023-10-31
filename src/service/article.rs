//! This module handles articles.

use crate::util::Oid;
use crate::util::{FromRow, PgResult};
use chrono::NaiveDateTime;
use futures_util::{Stream, StreamExt};
use std::fmt::Write;
use std::{fmt, iter};
use tokio_postgres::Row;

/// Structure representing an article.
pub struct Article {
	/// The article's id.
	pub id: Oid,
	/// Timestamp since epoch at which the article has been posted.
	///
	/// If `None`, the article has not been posted yet.
	pub post_date: Option<NaiveDateTime>,

	/// The the article's content.
	pub content: ArticleContent,
}

impl FromRow for Article {
	fn from_row(row: &Row) -> Self {
		let id = row.get("id");
		Self {
			id,
			post_date: row.get("post_date"),

			content: ArticleContent {
				article_id: id,
				edit_date: row.get("edit_date"),
				title: row.get("title"),
				description: row.get("description"),
				cover_url: row.get("cover_url"),
				content: row.get("content"),
				tags: row.get("tags"),
				public: row.get("public"),
				sponsor: row.get("sponsor"),
				comments_locked: row.get("comments_locked"),
			},
		}
	}
}

impl Article {
	/// Returns the list of articles.
	pub async fn list(db: &tokio_postgres::Client) -> PgResult<impl Stream<Item = Self>> {
		Ok(db
			.query_raw(
				"SELECT * FROM article INNER JOIN article_content ON article_content.id = article.content_id ORDER BY article.id DESC",
				iter::empty::<u32>(),
			)
			.await?
			.map(|r| Self::from_row(&r.unwrap())))
	}

	/// Returns the article with the given ID.
	///
	/// `id` is the ID of the article.
	pub async fn from_id(db: &tokio_postgres::Client, id: &Oid) -> PgResult<Option<Self>> {
		db
			.query_opt("SELECT * FROM article INNER JOIN article_content ON article_content.id = article.content_id WHERE article.id = $1", &[id])
			.await
			.map(|r| r.map(|r| FromRow::from_row(&r)))
	}

	/// Creates a new article.
	///
	/// On success, the function updates the article ID on the content.
	pub async fn create(db: &tokio_postgres::Client, content: &mut ArticleContent) -> PgResult<()> {
		let post_date = content.public.then_some(content.edit_date);
		let row = db.query_one(
			r"WITH
				aid AS (SELECT nextval(pg_get_serial_sequence('article', 'id'))),
				cid AS (
					INSERT INTO article_content (
						article_id,
						edit_date,
						title,
						description,
						cover_url,
						content,
						tags,
						public,
						sponsor,
						comments_locked
					) VALUES ((SELECT nextval FROM aid), $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING article_content.id
				)
			INSERT INTO article (id, post_date, content_id) VALUES ((SELECT nextval FROM aid), $1, (SELECT id FROM cid)) RETURNING article.id",
			&[
				&post_date,
				&content.edit_date,
				&content.title,
				&content.description,
				&content.cover_url,
				&content.content,
				&content.tags,
				&content.public,
				&content.sponsor,
				&content.comments_locked,
			]
		).await?;
		content.article_id = row.get(0);
		Ok(())
	}

	/// Edits the article's content.
	///
	/// `content` is the new content of the article.
	pub async fn edit(db: &tokio_postgres::Client, content: &ArticleContent) -> PgResult<()> {
		let post_date = content.public.then_some(content.edit_date);
		db.execute(r"WITH cid AS (
				INSERT INTO article_content (article_id, edit_date, title, description, cover_url, content, tags, public, sponsor, comments_locked)
					VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id
			)
			UPDATE article SET post_date = COALESCE(post_date, $11), content_id = (SELECT id FROM cid) WHERE id = $1",
			 &[
				 &content.article_id,
				 &content.edit_date,
				 &content.title,
				 &content.description,
				 &content.cover_url,
				 &content.content,
				 &content.tags,
				 &content.public,
				 &content.sponsor,
				 &content.comments_locked,
				 &post_date,
			])
			.await?;
		Ok(())
	}

	/// Write the HTML code representing the article's tags to the given output.
	///
	/// `admin` tells whether the user is admin.
	pub fn get_tags_html<W: Write>(&self, out: &mut W, admin: bool) -> fmt::Result {
		if admin {
			if self.content.public {
				write!(out, r#"<li class="tag">Public</li>"#)?;
			} else {
				write!(out, r#"<li class="tag">Private</li>"#)?;
			}
		}
		if self.content.sponsor {
			write!(out, "<i>Sponsors early access</i>&nbsp;&nbsp;&nbsp;❤️")?;
		}
		self.content
			.tags
			.split(',')
			.map(str::trim)
			.filter(|s| !s.is_empty())
			.try_for_each(|tag| write!(out, r#"<li class="tag">{tag}</li>"#))
	}
}

/// Content of an article.
///
/// Several contents are stored for the same article to keep the history of edits.
pub struct ArticleContent {
	/// The ID of the article.
	pub article_id: Oid,
	/// Timestamp since epoch at which the article has been edited.
	pub edit_date: NaiveDateTime,
	/// The article's title.
	pub title: String,
	/// The article's description.
	pub description: String,
	/// The URL to the cover image of the article.
	pub cover_url: String,
	/// The content of the article in markdown.
	pub content: String,
	/// The comma-separated list of tags on the article.
	pub tags: String,
	/// Tells whether the article is public.
	pub public: bool,
	/// Tells whether the article is reserved for sponsors.
	pub sponsor: bool,
	/// Tells whether comments are locked on the article.
	pub comments_locked: bool,
}

impl ArticleContent {
	/// Returns the URL title of the article.
	pub fn get_url_title(&self) -> String {
		self.title
			.chars()
			.filter_map(|c| match c {
				c if c.is_whitespace() => Some('-'),
				c if c.is_ascii() => Some(c),
				_ => None,
			})
			.collect::<String>()
			.to_lowercase()
	}

	/// Returns the path to the article.
	pub fn get_path(&self) -> String {
		format!("/a/{}/{}", self.article_id, self.get_url_title())
	}

	/// Returns the URL of the article.
	pub fn get_url(&self) -> String {
		format!("https://blog.lenot.re{}", self.get_path())
	}
}
