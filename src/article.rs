//! This module handles articles.

use crate::util;
use bson::oid::ObjectId;
use bson::Bson;
use chrono::DateTime;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use serde::Deserialize;
use serde::Serialize;

/// Structure representing an article.
#[derive(Serialize, Debug, Deserialize)]
pub struct Article {
	/// The article's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,
	/// The ID of the article's content.
	pub content_id: ObjectId,
	/// Timestamp since epoch at which the article has been posted.
	#[serde(with = "util::serde_date_time")]
	pub post_date: DateTime<Utc>,
}

impl Article {
	/// Returns the list of articles.
	///
	/// `db` is the database.
	pub async fn list(db: &mongodb::Database) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		let find_options = FindOptions::builder()
			.sort(Some(doc! {
				"post_date": -1
			}))
			.build();

		collection
			.find(doc! {}, Some(find_options))
			.await?
			.try_collect()
			.await
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
		let collection = db.collection::<Self>("article");
		collection.find_one(Some(doc! {"_id": id}), None).await
	}

	/// Inserts the current article in the database.
	///
	/// `db` is the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<Bson, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection
			.insert_one(self, None)
			.await
			.map(|r| r.inserted_id)
	}

	/// Updates the articles with the given ID.
	pub async fn update(
		db: &mongodb::Database,
		id: ObjectId,
		update: bson::Document,
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection
			.update_one(doc! { "_id": id }, doc! { "$set": update }, None)
			.await
			.map(|_| ())
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
#[derive(Serialize, Deserialize)]
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
	#[serde(with = "util::serde_date_time")]
	pub edit_date: DateTime<Utc>,
}

impl ArticleContent {
	/// Returns the article content with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the content.
	pub async fn from_id(
		db: &mongodb::Database,
		id: &ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article_content");
		collection.find_one(Some(doc! {"_id": id}), None).await
	}

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
		format!("/article/{}/{}", self.article_id, self.get_url_title())
	}

	/// Returns the URL of the article.
	pub fn get_url(&self) -> String {
		format!("https://blog.lenot.re{}", self.get_path())
	}
}
