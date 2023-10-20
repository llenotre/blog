//! This module handles articles.

use crate::util::Oid;
use crate::util::{FromRow, PgResult};
use chrono::NaiveDateTime;
use futures_util::{Stream, StreamExt};
use std::iter;
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
		Self {
			id: row.get("id"),
			post_date: row.get("post_date"),

			content: ArticleContent {
				article_id: row.get("article_content.article_id"),
				edit_date: row.get("article_content.edit_date"),
				title: row.get("article_content.title"),
				desc: row.get("article_content.desc"),
				cover_url: row.get("article_content.cover_url"),
				content: row.get("article_content.content"),
				tags: row.get("article_content.tags"),
				public: row.get("article_content.public"),
				sponsor: row.get("article_content.sponsor"),
				comments_locked: row.get("article_content.comments_locked"),
			},
		}
	}
}

impl Article {
	/// Returns the list of articles.
	pub async fn list(db: &tokio_postgres::Client) -> PgResult<impl Stream<Item = Self>> {
		Ok(db
			.query_raw(
				"SELECT * FROM article ORDER BY post_date DESC",
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
			.query_opt("SELECT * FROM article WHERE id = '$1'", &[id])
			.await
			.map(|r| r.map(|r| FromRow::from_row(&r)))
	}

	/// Edits the article's content.
	///
	/// Arguments:
	/// - `content` is the new content of the article.
	/// - `date` is the edit date.
	pub async fn edit(
		db: &tokio_postgres::Client,
		content: &ArticleContent,
		date: &NaiveDateTime,
	) -> PgResult<()> {
		let post_date = content.public.then_some(date);
		db.execute(r#"BEGIN TRANSACTION
			WITH cid AS (
				INSERT INTO article_content (article_id, edit_date, title, desc, cover_url, content, tags, public, sponsor, comments_locked)
					VALUES ($1, $2, $4, $5, $6, $7, $8, $9, $10, $11)
			);
			UPDATE article SET content_id = cid post_date = COALESCE(post_date, $3) WHERE id = $1;
		COMMIT"#, &[
			&content.article_id,
			date,
			&post_date,
			&content.title,
			&content.desc,
			&content.cover_url,
			&content.content,
			&content.tags,
			&content.public,
			&content.sponsor,
			&content.comments_locked,
		]).await?;
		Ok(())
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
	pub desc: String,
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
