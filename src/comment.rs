//! This module handles comments on articles.

use actix_session::Session;
use actix_web::{delete, error, get, post, web, HttpResponse, Responder};
use bson::doc;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use crate::GlobalData;
use crate::article::Article;
use crate::markdown;
use crate::user::User;
use crate::util;
use futures_util::stream::TryStreamExt;
use mongodb::options::FindOneOptions;
use serde::Deserialize;
use serde::Serialize;

/// The maximum length of a comment in characters.
pub const MAX_CHARS: usize = 10000;

// TODO support pinned comments

/// Structure representing a comment on an article.
#[derive(Debug, Serialize, Deserialize)]
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
	/// Returns a placeholder for a deleted comment.
	pub fn deleted(id: ObjectId) -> Self {
		Self {
			id,

			article: ObjectId::new(),
			response_to: None,

			author: ObjectId::new(),

			post_date: DateTime::<Utc>::default(),

			removed: true,
		}
	}

	/// Returns the list of comments for the article with the given id `article_id`.
	/// Comments are returns ordered by decreasing post date.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `not_removed` tells whether to the function must return only comments that are not
	/// removed.
	pub async fn list_for_article(
		db: &mongodb::Database,
		article_id: ObjectId,
		not_removed: bool,
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		let filter = if not_removed {
			doc! {
				"article": article_id,
				"removed": false,
			}
		} else {
			doc! {"article": article_id}
		};
		collection
			.find(Some(filter), None)
			.await?
			.try_collect()
			.await
	}

	/// Inserts the current comment in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		collection.insert_one(self, None).await.map(|_| ())
	}

	/// Deletes the comment with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `comment_id` is the ID of the comment to delete.
	/// - `user_id` is the ID of the user trying to delete the comment.
	/// - `bypass_perm` tells whether the function can bypass user's permissions.
	pub async fn delete(
		db: &mongodb::Database,
		comment_id: &ObjectId,
		user_id: &ObjectId,
		bypass_perm: bool,
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		let filter = if !bypass_perm {
			doc! {
				"_id": comment_id,
				"author": user_id,
			}
		} else {
			doc! {"_id": comment_id}
		};

		collection
			.update_one(filter, doc! {"$set": {"removed": true}}, None)
			.await?;

		Ok(())
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
			.sort(Some(doc! {
				"edit_date": -1
			}))
			.build();

		collection
			.find_one(
				Some(doc! {
					"comment_id": id,
				}),
				Some(find_options),
			)
			.await
	}

	/// Inserts the current content in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
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

/// The payload for the request allowing to post a comment.
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
	info: web::Json<PostCommentPayload>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let info = info.into_inner();

	if info.content.is_empty() {
		return Err(error::ErrorBadRequest(""));
	}
	if info.content.len() > MAX_CHARS {
		return Err(error::ErrorPayloadTooLarge(""));
	}

	let db = data.get_database();

	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Check article exists
	let article_id = ObjectId::parse_str(info.article_id).map_err(|_| error::ErrorNotFound(""))?;
	let article = Article::from_id(&db, &article_id).await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound(""));
	};
	if !article.public && !admin {
		return Err(error::ErrorNotFound(""));
	}

	let Some(user_id) = session.get::<String>("user_id").unwrap() else {
		return Err(error::ErrorForbidden(""));
	};
	let user_id = ObjectId::parse_str(&user_id).map_err(|_| error::ErrorBadRequest(""))?;

	let id = ObjectId::new();
	let date = chrono::offset::Utc::now();

	let comment = Comment {
		id,

		article: article_id,
		response_to: info.response_to,

		author: user_id,

		post_date: date,

		removed: false,
	};
	let comment_content = CommentContent {
		comment_id: id,

		edit_date: date,

		content: info.content,
	};

	// Insert comment
	comment_content
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	comment
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}

#[delete("/comment/{id}")]
pub async fn delete(
	data: web::Data<GlobalData>,
	comment_id: web::Path<String>,
	session: Session,
) -> impl Responder {
	let comment_id = comment_id.into_inner();
	let comment_id = ObjectId::parse_str(&comment_id).map_err(|_| error::ErrorBadRequest(""))?;

	let Some(user_id) = session.get::<String>("user_id").unwrap() else {
		return Err(error::ErrorForbidden(""));
	};
	let user_id = ObjectId::parse_str(&user_id).map_err(|_| error::ErrorBadRequest(""))?;

	let db = data.get_database();

	// Delete if the user has permission
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	Comment::delete(&db, &comment_id, &user_id, admin)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// TODO change status according to error (not found, forbidden, etc...)
	Ok(HttpResponse::Ok().finish())
}

#[get("/comment/preview")]
pub async fn preview(payload: String) -> impl Responder {
	let markdown = markdown::to_html(&payload, true);
	HttpResponse::Ok().body(markdown)
}
