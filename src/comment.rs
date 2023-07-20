//! This module handles comments on articles.

use crate::user::User;
use crate::util;
use actix_web::error;
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
pub const MAX_CHARS: usize = 5000;

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

/// Returns the HTML code for a comment editor.
///
/// Arguments:
/// - `user_login` is the handle of the logged user.
/// - `article` is the action to perform.
/// - `comment_id` is the ID of the comment for which the action is performed.
/// - `content` is the default content of the editor.
pub fn get_comment_editor(
	user_login: &str,
	action: &str,
	comment_id: Option<&str>,
	content: Option<&str>,
) -> String {
	let id = comment_id
		.map(|s| s.to_string())
		.unwrap_or("null".to_owned());
	let id_quoted = comment_id
		.map(|s| format!("'{}'", s))
		.unwrap_or("null".to_owned());
	let content = content.unwrap_or_default();

	format!(
		r#"<div class="comment-editor">
            <img class="comment-avatar" src="/avatar/{user_login}" />
            <textarea id="comment-{id}-{action}-content" name="content" placeholder="What are your thoughts?" onfocus="expand_editor('comment-{id}-{action}-content')" oninput="input({id_quoted}, '{action}')">{content}</textarea>
            <button id="comment-{id}-{action}-submit" onclick="{action}({id_quoted})">
                <i class="fa-regular fa-paper-plane"></i>
            </button>
        </div>
		<h6><span id="comment-{id}-{action}-len">0</span>/{MAX_CHARS} characters - Markdown is supported - Make sure you follow the <a href="/legal#conduct" target="_blank">Code of conduct</a> - <a href="/logout">Logout</a></h6>"#
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
/// - `article_title` is the title of the comment's article.
/// - `comment` is the comment.
/// - `replies` is the list of replies. If `None`, the comment itself is a reply.
/// - `user_id` is the ID of the current user. If not logged, the value is `None`.
/// - `user_login` is the handle of the logged user. If `None`, the user is not logged.
/// - `admin` tells whether the current user is admin.
#[async_recursion]
pub async fn comment_to_html(
	db: &mongodb::Database,
	article_title: &str,
	comment: &Comment,
	replies: Option<&'async_recursion [Comment]>,
	user_id: Option<&'async_recursion ObjectId>,
	user_login: Option<&'async_recursion str>,
	admin: bool,
) -> actix_web::Result<String> {
	let com_id = comment.id;

	// HTML for comment's replies
	let replies_html = match replies {
		Some(replies) => {
			let mut html = String::new();
			for com in replies {
				html.push_str(
					&comment_to_html(db, article_title, com, None, user_id, user_login, admin)
						.await?,
				);
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
		r##"<a href="#{com_id}" id="{com_id}-link" onclick="clipboard('{com_id}-link', 'https://blog.lenot.re/article/{article_id}/{article_title}#com-{com_id}')" class="comment-button" alt="Copy link"><i class="fa-solid fa-link"></i></a>"##,
		article_id = comment.article
	));
	if (user_id == Some(&comment.author) || admin) && !comment.removed {
		buttons.push(format!(
			r##"<a href="#comment-{com_id}-edit-content" class="comment-button" onclick="toggle_edit('{com_id}')"><i class="fa-solid fa-pen-to-square"></i></a>"##
		));
		buttons.push(format!(
			r##"<a class="comment-button" onclick="del('{com_id}')"><i class="fa-solid fa-trash"></i></a>"##
		));
	}
	if user_id.is_some() && replies.is_some() {
		buttons.push(format!(
			r##"<a href="#comment-{com_id}-post-content" class="comment-button" onclick="toggle_reply('{com_id}')"><i class="fa-solid fa-reply"></i></a>"##
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
	let html_url = ammonia::clean(&author.github_info.html_url);
	let login = ammonia::clean(&author.github_info.login);

	// Get content of comment
	let content = CommentContent::from_id(db, comment.content_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(content) = content else {
		return Ok(String::new());
	};
	let markdown = util::markdown_to_html(&content.content, true);

	let mut date_text = if content.edit_date > comment.post_date {
		format!(
			r#"<span id="date-long">{}</span> (edit: <span id="date-long">{}</span>)"#,
			comment.post_date.to_rfc3339(),
			content.edit_date.to_rfc3339()
		)
	} else {
		format!(
			r#"<span id="date-long">{}</span>"#,
			comment.post_date.to_rfc3339()
		)
	};
	if comment.removed && admin {
		date_text.push_str(" - REMOVED");
	}

	let (edit_editor, reply_editor) = match user_login {
		Some(user_login) => (
			get_comment_editor(
				user_login,
				"edit",
				Some(&com_id.to_hex()),
				Some(&content.content),
			),
			get_comment_editor(user_login, "post", Some(&com_id.to_hex()), None),
		),

		None => (String::new(), String::new()),
	};

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
