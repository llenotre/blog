//! This module handles articles.

use crate::util::PgResult;
use macros::FromRow;
use crate::util;
use chrono::DateTime;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;

/// Structure representing an article.
#[derive(Debug, FromRow)]
pub struct Article {
	/// The article's id.
	pub id: ObjectId,
	/// The ID of the article's content.
	pub content_id: ObjectId,
	/// Timestamp since epoch at which the article has been posted.
	pub post_date: Option<DateTime<Utc>>,
}

impl Article {
	/// Returns the list of articles.
	///
	/// `db` is the database.
	pub async fn list(db: &mongodb::Database) -> PgResult<Vec<Self>> {
        db.query("SELECT * FROM article ORDER BY post_date DESC").await
	}

	/// Returns the article with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the article.
	pub async fn from_id(
		db: &mongodb::Database,
		id: &ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
        db.query("SELECT * FROM article WHERE id = '$1'", &[id]).await
	}

	/// Inserts the current article in the database.
	///
	/// `db` is the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(&self, db: &mongodb::Database) -> PgResult<Bson> {
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
		db: &mongodb::Database,
		id: ObjectId,
		content_id: ObjectId,
		post_date: Option<DateTime<Utc>>,
	) -> Result<(), mongodb::error::Error> {
        db.execute("UPDATE article SET content_id = '$1' post_date = COALESCE(post_date, $2) WHERE id = '$3'", &[content_id, post_date, id]).await
	}

	/// Returns the article's content.
	pub async fn get_content(
		&self,
		db: &mongodb::Database,
	) -> Result<ArticleContent, mongodb::error::Error> {
		Ok(ArticleContent::from_id(db, &self.content_id)
			.await?
			.unwrap())
	}
}

/// Content of an article.
///
/// Several contents are stored for the same article to keep the history of edits.
#[derive(FromRow)]
pub struct ArticleContent {
	/// The ID of the article.
	pub article_id: ObjectId,

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

	/// Timestamp since epoch at which the article has been edited.
	pub edit_date: DateTime<Utc>,
}

impl ArticleContent {
	/// Inserts the current content in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<ObjectId, mongodb::error::Error> {
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
