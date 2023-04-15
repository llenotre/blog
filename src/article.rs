//! This module handles articles.

use bson::Bson;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use crate::util;
use futures_util::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use serde::Deserialize;
use serde::Serialize;

/// Structure representing an article.
#[derive(Serialize, Deserialize)]
pub struct Article {
	/// The article's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The article's title.
	pub title: String,
	/// The article's description.
	pub desc: String,
	/// Timestamp since epoch at which the article has been posted.
	#[serde(with = "util::serde_date_time")]
	pub post_date: DateTime<Utc>,
	/// Tells whether the article is public.
	pub public: bool,

	/// The article's content.
	pub content: String,
}

impl Article {
	/// Returns the total number of articles.
	pub async fn get_total_count(db: &mongodb::Database) -> Result<u32, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.count_documents(None, None)
			.await
			.map(|n| n as _)
	}

	/// Returns the list of articles for the given page.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `page` is the page number.
	/// - `per_page` is the number of articles per page.
	/// - `public` tells whether to the function must return only public articles.
	pub async fn list(
		db: &mongodb::Database,
		page: u32,
		per_page: u32,
		public: bool
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		let find_options = FindOptions::builder()
			.skip(Some((page * per_page) as _))
			.limit(Some(per_page as _))
			.sort(Some(doc!{
				"post_date": -1
			}))
			.build();

		let filter = if public {
			Some(doc!{
				"public": true,
			})
		} else {
			None
		};

		collection.find(
			filter,
			Some(find_options)
		)
			.await?
			.try_collect()
			.await
	}

	/// Returns the article with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the article.
	pub async fn get(
		db: &mongodb::Database,
		id: ObjectId
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.find_one(Some(doc!{"_id": id}), None).await
	}

	/// Inserts the current article in the database.
	///
	/// `db` is the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(
		&self,
		db: &mongodb::Database
	) -> Result<Bson, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.insert_one(self, None)
			.await
			.map(|r| r.inserted_id)
	}
}
