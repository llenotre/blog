//! This module handles comments on articles.

use crate::article::Article;
use crate::markdown;
use crate::user::User;
use crate::util;
use crate::GlobalData;
use actix_session::Session;
use actix_web::{delete, error, patch, post, web, HttpResponse, Responder};
use async_recursion::async_recursion;
use bson::doc;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

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

	/// The ID of the comment's content.
	pub content_id: ObjectId,

	/// Tells whether the comment has been removed.
	pub removed: bool,
}

impl Comment {
	/// Returns the comment with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the comment.
	pub async fn from_id(
		db: &mongodb::Database,
		id: &ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		collection.find_one(Some(doc! {"_id": id}), None).await
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

	/// Updates the ID of the comment's content.
	pub async fn update_content(
		&self,
		db: &mongodb::Database,
		content_id: ObjectId,
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("comment");
		collection
			.update_one(
				doc! {"_id": self.id},
				doc! {"$set": {"content_id": content_id}},
				None,
			)
			.await
			.map(|_| ())
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
	pub async fn from_id(
		db: &mongodb::Database,
		id: ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment_content");
		collection
			.find_one(
				Some(doc! {
					"_id": id,
				}),
				None,
			)
			.await
	}

	/// Inserts the current content in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<ObjectId, mongodb::error::Error> {
		let collection = db.collection::<Self>("comment_content");
		collection
			.insert_one(self, None)
			.await
			.map(|r| r.inserted_id.as_object_id().unwrap())
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

/// Returns the HTML code for a comment editor.
///
/// Arguments:
/// - `article` is the action to perform.
/// - `comment_id` is the ID of the comment for which the action is performed.
/// - `content` is the default content of the editor.
pub fn get_comment_editor(action: &str, comment_id: Option<&str>, content: Option<&str>) -> String {
	let id = comment_id
		.map(|s| format!("{}", s))
		.unwrap_or("null".to_owned());
	let id_quoted = comment_id
		.map(|s| format!("'{}'", s))
		.unwrap_or("null".to_owned());
	let content = content.unwrap_or_default();

	format!(
		r#"<div class="comment-editor">
            <textarea id="comment-{id}-{action}-content" name="content" placeholder="What are your thoughts?" onfocus="expand_editor('comment-{id}-{action}-content')" oninput="input({id_quoted})">{content}</textarea>
            <button id="comment-{id}-{action}-submit" onclick="{action}({id_quoted})">
                <i class="fa-regular fa-paper-plane"></i>
            </button>
        </div>
		<h6><span id="comment-{id}-len">0</span>/{MAX_CHARS} characters - Markdown is supported - Make sure you follow the <a href="/legal#conduct" target="_blank">Code of conduct</a></h6>"#
	)
}

/// Groups all comments into a list of comment-replies pairs.
pub fn group_comments(comments: Vec<Comment>) -> Vec<(Comment, Vec<Comment>)> {
	let mut base = HashMap::new();
	let mut replies = Vec::new();

	// Partition comments
	for com in comments {
		if com.response_to.is_none() {
			base.insert(com.id, (com, vec![]));
		} else {
			replies.push(com);
		}
	}

	// Assign replies to comments
	for reply in replies {
		let base_id = reply.response_to.as_ref().unwrap();

		if let Some(b) = base.get_mut(base_id) {
			b.1.push(reply);
		}
		// If the base comment doesn't exist, discard the reply
	}

	let mut comments: Vec<_> = base.into_values().collect();
	comments.sort_unstable_by(|c0, c1| c0.0.post_date.cmp(&c1.0.post_date));
	for c in &mut comments {
		c.1.sort_unstable_by(|c0, c1| c0.post_date.cmp(&c1.post_date));
	}

	comments
}

/// Returns the HTML for the given comment-replies pair.
///
/// Arguments:
/// - `db` is the database.
/// - `comment` is the comment.
/// - `replies` is the list of replies. If `None`, the comment itself is a reply.
/// - `user_id` is the ID of the current user. If not logged, the value is `None`.
/// - `article_id` is the ID of the current article.
/// - `admin` tells whether the current user is admin.
#[async_recursion]
pub async fn comment_to_html(
	db: &mongodb::Database,
	comment: &Comment,
	replies: Option<&'async_recursion [Comment]>,
	user_id: Option<&'async_recursion ObjectId>,
	article_id: &ObjectId,
	admin: bool,
) -> actix_web::Result<String> {
	let com_id = comment.id;

	// HTML for comment's replies
	let replies_html = match replies {
		Some(replies) => {
			let mut html = String::new();
			for com in replies {
				html.push_str(&comment_to_html(db, com, None, user_id, article_id, admin).await?);
			}

			format!(
				r#"<div id="comment-{com_id}-replies" class="comments-list" style="margin-top: 20px;">
				{html}
			</div>"#
			)
		}
		None => String::new(),
	};

	// HTML for comment's buttons
	let mut buttons = Vec::with_capacity(4);
	buttons.push(format!(
		r##"<a href="#{com_id}" id="{com_id}-link" onclick="clipboard('{com_id}-link', 'https://blog.lenot.re/article/{article_id}#com-{com_id}')" class="comment-button" alt="Copy link"><i class="fa-solid fa-link"></i></a>"##
	));
	if (user_id == Some(&comment.author) || admin) && !comment.removed {
		buttons.push(format!(
			r#"<a class="comment-button" onclick="toggle_edit('{com_id}')"><i class="fa-solid fa-pen-to-square"></i></a>"#
		));
		buttons.push(format!(
			r#"<a class="comment-button" onclick="del('{com_id}')"><i class="fa-solid fa-trash"></i></a>"#
		));
	}
	if user_id.is_some() && replies.is_some() {
		buttons.push(format!(
			r#"<a class="comment-button" onclick="toggle_reply('{com_id}')"><i class="fa-solid fa-reply"></i></a>"#
		));
	}
	let buttons_html = if !buttons.is_empty() {
		let buttons_html: String = buttons.into_iter().collect();
		format!(
			r#"<div class="comment-buttons">
				{buttons_html}
			</div>"#
		)
	} else {
		String::new()
	};

	if comment.removed && !admin {
		return Ok(format!(
			r##"<div class="comment">
				<div class="comment-header">
					<p><i class="fa-solid fa-trash"></i>&nbsp;<i>deleted comment</i></p>
				</div>
				<div class="comment-content">
					{buttons_html}
				</div>
				{replies_html}
			</div>"##
		));
	}

	// Get author
	let author = User::from_id(db, comment.author)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(author) = author else {
		return Ok(String::new());
	};
	let html_url = author.github_info.html_url;
	let login = author.github_info.login;

	// Get content of comment
	let content = CommentContent::from_id(db, comment.content_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(content) = content else {
		return Ok(String::new());
	};
	let markdown = markdown::to_html(&content.content, true);

	let mut date_text = if content.edit_date > comment.post_date {
		format!(
			"{} UTC (edit: {} UTC)",
			comment.post_date.format("%d/%m/%Y %H:%M:%S"),
			content.edit_date.format("%d/%m/%Y %H:%M:%S")
		)
	} else {
		format!("{} UTC", comment.post_date.format("%d/%m/%Y %H:%M:%S"))
	};
	if comment.removed && admin {
		date_text.push_str(" - REMOVED");
	}

	let edit_editor = get_comment_editor("edit", Some(&com_id.to_hex()), Some(&content.content));
	let reply_editor = get_comment_editor("post", Some(&com_id.to_hex()), None);

	Ok(format!(
		r##"<div class="comment" id="com-{com_id}">
			<div class="comment-header">
				<div>
				<a href="{html_url}" target="_blank"><img class="comment-avatar" src="/avatar/{login}"></img></a>
				</div>
				<div>
					<p><a href="{html_url}" target="_blank">{login}</a></p>
				</div>
                <div>
					<h6 style="color: gray;">{date_text}</h6>
                </div>
				<div>
					{buttons_html}
				</div>
			</div>
			<div class="comment-content">
				{markdown}
				<div id="reaction-{com_id}-selector" class="reaction-selector" hidden>
					<a class="reaction-button">👍</a>
					<a class="reaction-button">👎</a>
					<a class="reaction-button">😀</a>
					<a class="reaction-button">🙃</a>
					<a class="reaction-button">❤️</a>
					<a class="reaction-button">🥳</a>
					<a class="reaction-button">😕</a>
					<a class="reaction-button">☹️</a>
					<a class="reaction-button">💩</a>
				</div>
				<div class="comment-buttons">
					<a class="comment-button" onclick="toggle_reactions('reaction-{com_id}-selector')"><i class="fa-solid fa-face-smile"></i></a>
					TODO
				</div>
			</div>
			<div id="editor-{com_id}-edit" hidden>
				<p>Edit comment</p>
				{edit_editor}
			</div>
			<div id="editor-{com_id}-reply" hidden>
				<p>Reply</p>
				{reply_editor}
			</div>
			{replies_html}
		</div>"##
	))
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
	let article = Article::from_id(&db, &article_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound(""));
	};
	let article_content = article
		.get_content(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !article_content.public && !admin {
		return Err(error::ErrorNotFound(""));
	}

	let Some(user_id) = session.get::<String>("user_id").unwrap() else {
		return Err(error::ErrorForbidden(""));
	};
	let user_id = ObjectId::parse_str(&user_id).map_err(|_| error::ErrorBadRequest(""))?;

	let id = ObjectId::new();
	let date = chrono::offset::Utc::now();

	// Insert comment content
	let comment_content = CommentContent {
		comment_id: id,

		edit_date: date,

		content: info.content,
	};
	let content_id = comment_content
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let comment = Comment {
		id,

		article: article_id,
		response_to: info.response_to,
		author: user_id,
		post_date: date,

		content_id,

		removed: false,
	};
	comment
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().finish())
}

/// The payload for the request allowing to edit a comment.
#[derive(Deserialize)]
pub struct EditCommentPayload {
	/// The ID of the comment.
	comment_id: String,

	/// The new content of the comment in markdown.
	content: String,
}

#[patch("/comment")]
pub async fn edit(
	data: web::Data<GlobalData>,
	info: web::Json<EditCommentPayload>,
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

	let Some(user_id) = session.get::<String>("user_id").unwrap() else {
		return Err(error::ErrorForbidden(""));
	};
	let user_id = ObjectId::parse_str(&user_id).map_err(|_| error::ErrorBadRequest(""))?;

	// Check comment exists
	let comment_id = ObjectId::parse_str(info.comment_id).map_err(|_| error::ErrorNotFound(""))?;
	let comment = Comment::from_id(&db, &comment_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(comment) = comment else {
		return Err(error::ErrorNotFound(""));
	};

	if !admin && comment.author != user_id {
		return Err(error::ErrorForbidden(""));
	}

	// Insert comment content
	let date = chrono::offset::Utc::now();
	let comment_content = CommentContent {
		comment_id,

		edit_date: date,

		content: info.content,
	};
	let content_id = comment_content
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Update comment's content
	comment
		.update_content(&db, content_id)
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
