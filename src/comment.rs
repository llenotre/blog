//! This module handles comments on articles.

use actix_session::Session;
use actix_web::{
	Responder,
	post,
	web,
};
use bson::doc;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use crate::GlobalData;
use crate::user;
use crate::util;
use futures_util::stream::TryStreamExt;
use mongodb::options::FindOneOptions;
use mongodb::options::FindOptions;
use serde::Deserialize;
use serde::Serialize;

// TODO support pinned comments

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

	/// The ID of author of the comment.
	pub author: ObjectId,

	/// Timestamp since epoch at which the comment has been posted.
	#[serde(with = "util::serde_date_time")]
	pub post_date: DateTime<Utc>,

	/// Tells whether the comment has been removed.
	pub removed: bool,
}

impl Comment {
	/// Returns the list of comments for the article with the given id `article_id`.
	/// Comments are returns ordered by decreasing post date.
	///
	/// `db` is the database.
	pub async fn list_for_article(
		db: &mongodb::Database,
		article_id: ObjectId,
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		let options = FindOptions::builder()
			.sort(doc!{ "post_date": -1 })
			.build();
		collection.find(
			Some(doc!{
				"article": article_id,
				"removed": false,
			}),
			Some(options)
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
	pub comment_id: ObjectId,

	/// Timestamp since epoch at which the comment has been edited.
	#[serde(with = "util::serde_date_time")]
	pub edit_date: DateTime<Utc>,

	/// The content of the comment.
	pub content: String,
}

impl CommentContent {
	/// Returns the latest content of the comment with the given ID `id`.
	///
	/// `db` is the database.
	pub async fn get_for(
		db: &mongodb::Database,
		id: ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment_content");
		let find_options = FindOneOptions::builder()
			.sort(Some(doc!{
				"edit_date": -1
			}))
			.build();

		collection.find_one(
			Some(doc!{
				"comment_id": id,
			}),
			Some(find_options)
		)
			.await

	}

	/// Inserts the current content in the database.
	///
	/// `db` is the database.
	pub async fn insert(
		&self,
		db: &mongodb::Database
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("comment_content");
		collection.insert_one(self, None).await.map(|_| ())
	}
}

/// Reaction to an article or a comment.
#[derive(Serialize, Deserialize)]
pub struct Reaction {
	/// The ID of the article.
	pub article_id: Option<ObjectId>,
	/// The ID of the comment.
	pub comment_id: Option<ObjectId>,

	/// The ID of author of the reaction.
	pub author: ObjectId,

	/// The reaction.
	pub reaction: char,

	/// Reaction timestamp.
	#[serde(with = "util::serde_date_time")]
	pub edit_date: DateTime<Utc>,

	/// Tells whether the reaction has been removed.
	pub removed: bool,
}

/// TODO doc
#[derive(Deserialize)]
pub struct PostCommentPayload {
	/// The ID of the article.
	article_id: String,
	/// The ID of the comment this comment responds to. If `None`, this comment is not a response.
	response_to: Option<ObjectId>,

	/// The content of the comment in markdown.
	content: String,
}

// TODO error if article's comments are locked
#[post("/comment")]
pub async fn post(
	data: web::Data<GlobalData>,
	form: web::Form<PostCommentPayload>,
	session: Session,
) -> impl Responder {
	let form = form.into_inner();
	let article_id = ObjectId::parse_str(form.article_id).unwrap(); // TODO handle error (http 404)

	let Some(user_id) = session.get::<String>("user_id").unwrap() else { // TODO handle error
		// TODO
		todo!();
	};
	let user_id = ObjectId::parse_str(&user_id).unwrap(); // TODO handle error

	let id = ObjectId::new();
	let date = chrono::offset::Utc::now();

	let comment = Comment {
		id,

		article: article_id,
		response_to: form.response_to,

		author: user_id,

		post_date: date,

		removed: false,
	};
	let comment_content = CommentContent {
		comment_id: id,

		edit_date: date,

		content: form.content,
	};

	// Insert comment
	let db = data.get_database();
	comment_content.insert(&db)
		.await
		.unwrap(); // TODO handle error (http 500)
	comment.insert(&db)
		.await
		.unwrap(); // TODO handle error (http 500)

	user::redirect_to_last_article(&session)
}
