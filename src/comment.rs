//! This module handles comments on articles.

use bson::doc;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use crate::util;
use futures_util::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;

/// Structure representing a comment on an article.
#[derive(Serialize, Deserialize)]
pub struct Comment {
	/// The comment's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The ID of the article.
	pub article: ObjectId,
	/// The ID of the comment this comment responds to. If `None`, this comment is not a response.
	pub response_to: Option<ObjectId>,

	/// The author of the comment.
	pub author: String,

	/// Timestamp since epoch at which the comment has been posted.
	#[serde(with = "util::serde_date_time")]
	pub post_date: DateTime<Utc>,

	/// Tells whether the comment has been removed.
	pub removed: bool,
}

impl Comment {
	/// Returns the list of comments for the article with the given id `article_id`.
	///
	/// `db` is the database.
	pub async fn list_for_article(
		db: &mongodb::Database,
		article_id: ObjectId,
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		collection.find(
			Some(doc!{
				"article_id": article_id,
				"removed": false,
			}),
			None
		)
			.await?
			.try_collect()
			.await

	}

	/// Inserts the current comment in the database.
	///
	/// `db` is the database.
	pub async fn insert(
		&self,
		db: &mongodb::Database
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		collection.insert_one(self, None).await.map(|_| ())
	}
}

/// Content of a comment.
///
/// Several contents are stored for the same comment to keep the history of edits.
#[derive(Serialize, Deserialize)]
pub struct CommentContent {
	/// The ID of the comment.
	pub comment: ObjectId,

	/// Timestamp since epoch at which the comment has been edited.
	#[serde(with = "util::serde_date_time")]
	pub edit_date: DateTime<Utc>,
}

/// Reaction to an article or a comment.
#[derive(Serialize, Deserialize)]
pub struct Reaction {
	/// The ID of the article.
	pub article_id: Option<ObjectId>,
	/// The ID of the comment.
	pub comment_id: Option<ObjectId>,

	/// The author of the reaction.
	pub author: String,

	/// The reaction.
	pub reaction: char,

	/// Reaction timestamp.
	#[serde(with = "util::serde_date_time")]
	pub edit_date: DateTime<Utc>,

	/// Tells whether the reaction has been removed.
	pub removed: bool,
}
