//! This module handles articles.

use crate::util::Oid;
use crate::util;
use crate::util::{FromRow, PgResult};
use chrono::DateTime;
use chrono::Utc;
use futures_util::{StreamExt, TryStreamExt};
use macros::FromRow;

/// Structure representing an article.
#[derive(FromRow)]
pub struct Article {
	/// The article's id.
	pub id: Oid,
	/// Timestamp since epoch at which the article has been posted.
	pub post_date: Option<DateTime<Utc>>,

	/// The the article's content.
	pub content: ArticleContent,
}

impl Article {
	/// Returns the list of articles.
	pub async fn list(db: &tokio_postgres::Client) -> PgResult<Vec<Self>> {
		db.query_raw("SELECT * FROM article ORDER BY post_date DESC", &[])
			.await?
			.map(|r| Self::from_row(&r.unwrap()))
			.try_collect()
			.await
	}

	/// Returns the article with the given ID.
	///
	/// `id` is the ID of the article.
	pub async fn from_id(db: &tokio_postgres::Client, id: &Oid) -> PgResult<Option<Self>> {
		Ok(db
			.query_opt("SELECT * FROM article WHERE id = '$1'", &[id])
			.await
			.map(|r| r.map(|r| FromRow::from_row(&r).unwrap()))?)
	}

	/// Inserts the current article in the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(&self, db: &tokio_postgres::Client) -> PgResult<Oid> {
		let collection = db.collection::<Self>("article");
		collection
			.insert_one(self, None)
			.await
			.map(|r| r.inserted_id)
	}

	/// Updates the articles with the given ID.
	///
	/// Arguments:
	/// - `content_id` is the ID of the article's new content.
	/// - `post_date` is the post date. It is updated if set and only at the first call.
	pub async fn update(
		db: &tokio_postgres::Client,
		id: Oid,
		content_id: Oid,
		post_date: Option<DateTime<Utc>>,
	) -> PgResult<()> {
		db.execute("UPDATE article SET content_id = '$1' post_date = COALESCE(post_date, $2) WHERE id = '$3'", &[content_id, post_date, id]).await?;
		Ok(())
	}
}

/// Content of an article.
///
/// Several contents are stored for the same article to keep the history of edits.
#[derive(FromRow)]
pub struct ArticleContent {
	/// The ID of the article.
	pub article_id: Oid,
	/// Timestamp since epoch at which the article has been edited.
	pub edit_date: DateTime<Utc>,
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
	/// Inserts the current content in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &tokio_postgres::Client) -> PgResult<Oid> {
		let collection = db.collection::<Self>("article_content");
		collection
			.insert_one(self, None)
			.await
			.map(|r| r.inserted_id.as_object_id().unwrap())
	}

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
		let id = util::encode_id(&self.article_id);
		format!("/a/{id}/{}", self.get_url_title())
	}

	/// Returns the URL of the article.
	pub fn get_url(&self) -> String {
		format!("https://blog.lenot.re{}", self.get_path())
	}
}
