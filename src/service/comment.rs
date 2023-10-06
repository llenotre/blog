//! This module handles comments on articles.

use crate::service::user::User;
use crate::util;
use actix_web::error;
use async_recursion::async_recursion;
use chrono::DateTime;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use crate::util::PgResult;

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
	/// The ID of the comment this comment replies to. If `None`, this comment is not a reply.
	pub reply_to: Option<ObjectId>,
	/// The ID of author of the comment.
	pub author: ObjectId,
	/// Timestamp since epoch at which the comment has been posted.
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
		db: &Database,
		id: &ObjectId,
	) -> PgResult<Option<Self>> {
        db.execute("SELECT * FROM comment WHERE id = '$1'", &[id]).await
	}

	/// Returns the list of comments for the article with the given id `article_id`.
	/// Comments are returns ordered by decreasing post date.
	///
	/// `db` is the database.
	pub async fn list_for_article(
		db: &Database,
		article_id: ObjectId,
	) -> PgResult<Vec<Self>> {
        db.execute("SELECT * FROM comment WHERE article = '$1'", &[article_id]).await
	}

	/// Returns replies to the current comment.
	pub async fn get_replies(&self, db: &Database) -> PgResult<Vec<Self>> {
        db.execute("SELECT * FROM comment WHERE reply_to = '$1'", &[self.id]).await
	}

	/// Inserts the current comment in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &Database) -> PgResult<()> {
		let collection = db.collection::<Self>("comment");
		collection.insert_one(self, None).await.map(|_| ())
	}

	/// Updates the ID of the comment's content.
	pub async fn update_content(
		&self,
		db: &Database,
		content_id: ObjectId,
	) -> PgResult<()> {
        db.execute("UPDATE comment SET content_id = '$1' WHERE id = '$2'", &[content_id, self.id]).await
	}

	/// Deletes the comment with the given ID.
	///
	/// Arguments:
	/// - `comment_id` is the ID of the comment to delete.
	/// - `user_id` is the ID of the user trying to delete the comment.
	/// - `bypass_perm` tells whether the function can bypass user's permissions.
	pub async fn delete(
		db: &Database,
		comment_id: &ObjectId,
		user_id: &ObjectId,
		bypass_perm: bool,
	) -> PgResult<()> {
        let now = Utc::now();
        if bypass_perm {
            db.execute("UPDATE comment SET remove_date = '$1' WHERE id = '$2'", &[now, comment_id]).await
        } else {
            db.execute("UPDATE comment SET remove_date = '$1' WHERE id = '$2' AND author = '$3'", &[now, comment_id, user_id]).await
        }
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
	pub edit_date: DateTime<Utc>,
	/// The content of the comment.
	pub content: String,
}

impl CommentContent {
	/// Inserts the current content in the database.
	pub async fn insert(&self, db: &Database) -> PgResult<ObjectId> {
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
pub fn get_editor(
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
            <a href="https://github.com/{user_login}" target="_blank"><img class="comment-avatar" src="/avatar/{user_login}" /></a>
            <textarea id="comment-{id}-{action}-content" name="content" placeholder="What are your thoughts?" onfocus="expand_editor('comment-{id}-{action}-content')" oninput="input({id_quoted}, '{action}')">{content}</textarea>
            <button id="comment-{id}-{action}-submit" onclick="{action}({id_quoted})">
                <i class="fa-regular fa-paper-plane"></i>
            </button>
        </div>
		<h6><span id="comment-{id}-{action}-len">0</span>/{MAX_CHARS} characters - Markdown is supported - Make sure you follow the <a href="/legal#conduct" target="_blank">Code of conduct</a> - <a href="/logout">Logout</a></h6>"#
	)
}

/// Groups all comments into a list of comment-replies pairs.
pub fn group(comments: Vec<Comment>) -> Vec<(Comment, Vec<Comment>)> {
	let mut base = HashMap::new();
	let mut replies = Vec::new();

	// Partition comments
	for com in comments {
		if com.reply_to.is_none() {
			base.insert(com.id, (com, vec![]));
		} else {
			replies.push(com);
		}
	}

	// Assign replies to comments
	for reply in replies {
		let base_id = reply.reply_to.as_ref().unwrap();

		if let Some(b) = base.get_mut(base_id) {
			b.1.push(reply);
		}
		// If the base comment doesn't exist, discard the reply
	}

	let mut comments: Vec<_> = base.into_values().collect();
	comments.sort_by(|c0, c1| c0.0.post_date.cmp(&c1.0.post_date));
	for c in &mut comments {
		c.1.sort_by(|c0, c1| c0.post_date.cmp(&c1.post_date));
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
pub async fn to_html(
	db: &Database,
	article_title: &str,
	comment: &Comment,
	replies: Option<&'async_recursion [Comment]>,
	user_id: Option<&'async_recursion ObjectId>,
	user_login: Option<&'async_recursion str>,
	admin: bool,
) -> actix_web::Result<String> {
	let com_id = util::encode_id(&comment.id);
	let article_id = util::encode_id(&comment.article);

	// HTML for comment's replies
	let replies_html = match replies {
		Some(replies) => {
			let mut html = String::new();
			for com in replies {
				html.push_str(
					&to_html(db, article_title, com, None, user_id, user_login, admin).await?,
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
	if !comment.removed {
		buttons.push(format!(
			r##"<a href="#{com_id}" id="{com_id}-link" onclick="clipboard('{com_id}-link', 'https://blog.lenot.re/a/{article_id}/{article_title}#com-{com_id}')" class="comment-button" alt="Copy link"><i class="fa-solid fa-link"></i></a>"##,
		));
	}
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
					{buttons_html}
				</div>
				<div class="comment-content">
					<p><i class="fa-solid fa-trash"></i>&nbsp;<i>deleted comment</i></p>
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
			get_editor(user_login, "edit", Some(&com_id), Some(&content.content)),
			get_editor(user_login, "post", Some(&com_id), None),
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
