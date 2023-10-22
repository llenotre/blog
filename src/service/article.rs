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
				"SELECT article.*,B1.*
					FROM article
					LEFT JOIN article_content AS B1 ON B1.article_id = article.id
					LEFT JOIN article_content AS B2 ON B2.article_id = article.id AND B2.edit_date > B1.edit_date
					WHERE B2.article_id IS NULL",
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
			.query_opt("SELECT * FROM article INNER JOIN article_content ON article_content.article_id = article.id WHERE id = $1 ORDER BY edit_date DESC LIMIT 1", &[id])
			.await
			.map(|r| r.map(|r| FromRow::from_row(&r)))
	}

	/// Creates a new article.
	///
	/// On success, the function updates the article ID on the content.
	pub async fn create(db: &tokio_postgres::Client, content: &mut ArticleContent) -> PgResult<()> {
		let row = db
			.query_one(
				"INSERT INTO article (post_date) VALUES ($1) RETURNING id",
				&[&content.edit_date],
			)
			.await?;
		let article_id: Oid = row.get("id");
		db.execute(
			r#"INSERT INTO article_content (
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
				) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
			&[
				&article_id,
				&content.edit_date,
				&content.title,
				&content.description,
				&content.cover_url,
				&content.content,
				&content.tags,
				&content.public,
				&content.sponsor,
				&content.comments_locked,
			],
		)
		.await?;
		content.article_id = article_id;
		Ok(())
	}

	/// Edits the article's content.
	///
	/// `content` is the new content of the article.
	pub async fn edit(db: &tokio_postgres::Client, content: &ArticleContent) -> PgResult<()> {
		// TODO transaction?
		db.execute("INSERT INTO article_content (article_id, edit_date, title, description, cover_url, content, tags, public, sponsor, comments_locked)
				VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
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
			])
			.await?;
		let post_date = content.public.then_some(content.edit_date);
		db.execute("UPDATE article SET post_date = COALESCE(post_date, $2) WHERE id = $1", &[&content.article_id, &post_date]).await?;
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
