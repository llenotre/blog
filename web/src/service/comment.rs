//! This module handles comments on articles.

use crate::service::user::User;
use crate::util;
use crate::util::{now, PgResult};
use crate::util::{FromRow, Oid};
use actix_web::error;
use async_recursion::async_recursion;
use chrono::NaiveDateTime;
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use tokio_postgres::Row;

/// The maximum length of a comment in characters.
pub const MAX_CHARS: usize = 5000;

// TODO support pinned comments

/// Structure representing a comment on an article.
pub struct Comment {
	/// The comment's id.
	pub id: Oid,

	/// The ID of the article.
	pub article_id: Oid,
	/// The ID of the comment this comment replies to. If `None`, this comment is not a reply.
	pub reply_to: Option<Oid>,
	/// The ID of author of the comment.
	pub author_id: Oid,
	/// Timestamp since epoch at which the comment has been posted.
	pub post_date: NaiveDateTime,

	/// The comment's content.
	pub content: CommentContent,

	/// Tells whether the comment has been removed.
	pub remove_date: Option<NaiveDateTime>,
}

impl FromRow for Comment {
	fn from_row(row: &Row) -> Self {
		let id = row.get("id");
		Self {
			id,

			article_id: row.get("article_id"),
			reply_to: row.get("reply_to"),
			author_id: row.get("author_id"),
			post_date: row.get("post_date"),

			content: CommentContent {
				comment_id: id,
				edit_date: row.get("edit_date"),
				content: row.get("content"),
			},

			remove_date: row.get("remove_date"),
		}
	}
}

impl Comment {
	/// Creates a comment.
	///
	/// Arguments:
	/// - `article_id` is the ID of the article associated with the comment.
	/// - `reply_to` is the comment to which the newly created comment replies.
	/// - `user` is the user posting the comment.
	/// - `post_date` is the date at which the comment has been posted.
	/// - `content` is the content of the comment in markdown.
	pub async fn create(
		db: &tokio_postgres::Client,
		article_id: &Oid,
		reply_to: &Option<Oid>,
		user_id: &Oid,
		post_date: &NaiveDateTime,
		content: &str,
	) -> PgResult<Oid> {
		let row = db
			.query_one(
				r#"WITH
				com_id AS (SELECT nextval(pg_get_serial_sequence('comment', 'id'))),
				cont_id AS (
					INSERT INTO comment_content (comment_id, edit_date, content)
						VALUES ((SELECT nextval FROM com_id), $4, $5)
						RETURNING comment_content.id
				)
			INSERT INTO comment (id, article_id, reply_to, author_id, post_date, content_id)
				VALUES ((SELECT nextval FROM com_id), $1, $2, $3, $4, (SELECT id FROM cont_id))
				RETURNING comment.id"#,
				&[article_id, reply_to, &user_id, post_date, &content],
			)
			.await?;
		let id = row.get(0);
		User::update_cooldown(db, user_id, post_date).await?;
		Ok(id)
	}

	/// Returns the comment with the given ID.
	///
	/// `id` is the ID of the comment.
	pub async fn from_id(db: &tokio_postgres::Client, id: &Oid) -> PgResult<Option<Self>> {
		Ok(db
			.query_opt("SELECT * FROM comment INNER JOIN comment_content ON comment_content.id = comment.content_id WHERE comment.id = $1", &[id])
			.await?
			.as_ref()
			.map(FromRow::from_row))
	}

	/// Returns the list of comments for the article with the given id `article_id`.
	///
	/// Comments are returns ordered by decreasing post date.
	pub async fn list_for_article(
		db: &tokio_postgres::Client,
		article_id: &Oid,
	) -> PgResult<impl Stream<Item = Self>> {
		Ok(db
			.query_raw("SELECT * FROM comment INNER JOIN comment_content ON comment_content.id = comment.content_id WHERE article_id = $1", &[article_id])
			.await?
			.map(|r| FromRow::from_row(&r.unwrap())))
	}

	/// Returns replies to the current comment.
	pub async fn get_replies(
		&self,
		db: &tokio_postgres::Client,
	) -> PgResult<impl Stream<Item = Self>> {
		Ok(db
			.query_raw("SELECT * FROM comment INNER JOIN comment_content ON comment_content.id = comment.content_id WHERE reply_to = $1", &[&self.id])
			.await?
			.map(|r| FromRow::from_row(&r.unwrap())))
	}

	/// Edits the comment's content.
	pub async fn edit(
		db: &tokio_postgres::Client,
		user_id: &Oid,
		content: &CommentContent,
	) -> PgResult<()> {
		db.execute(
			r#"WITH cid AS (
				INSERT INTO comment_content (comment_id, edit_date, content) VALUES ($1, $2, $3) RETURNING id
			)
			UPDATE comment SET content_id = (SELECT id FROM cid) WHERE id = $1"#,
			&[&content.comment_id, &content.edit_date, &content.content],
		)
		.await?;
		User::update_cooldown(db, user_id, &content.edit_date).await?;
		Ok(())
	}

	/// Deletes the comment with the given ID.
	///
	/// Arguments:
	/// - `comment_id` is the ID of the comment to delete.
	/// - `user_id` is the ID of the user trying to delete the comment.
	/// - `bypass_perm` tells whether the function can bypass user's permissions.
	pub async fn delete(
		db: &tokio_postgres::Client,
		comment_id: &Oid,
		user_id: &Oid,
		bypass_perm: bool,
	) -> PgResult<()> {
		let now = now();
		if bypass_perm {
			db.execute(
				"UPDATE comment SET remove_date = $1 WHERE id = $2",
				&[&now, comment_id],
			)
			.await?;
		} else {
			db.execute(
				"UPDATE comment SET remove_date = $1 WHERE id = $2 AND author = $3",
				&[&now, comment_id, user_id],
			)
			.await?;
		}
		Ok(())
	}
}

/// Content of a comment.
///
/// Several contents are stored for the same comment to keep the history of edits.
pub struct CommentContent {
	/// The ID of the comment.
	pub comment_id: Oid,
	/// Timestamp since epoch at which the comment has been edited.
	pub edit_date: NaiveDateTime,
	/// The content of the comment.
	pub content: String,
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
	comment_id: Option<Oid>,
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
	db: &tokio_postgres::Client,
	article_title: &str,
	comment: &Comment,
	replies: Option<&'async_recursion [Comment]>,
	user_id: Option<&'async_recursion Oid>,
	user_login: Option<&'async_recursion str>,
	admin: bool,
) -> actix_web::Result<String> {
	let com_id = comment.id;
	let article_id = comment.article_id;

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
	if comment.remove_date.is_none() {
		buttons.push(format!(
			r##"<a href="#{com_id}" id="{com_id}-link" onclick="clipboard('{com_id}-link', 'https://blog.lenot.re/a/{article_id}/{article_title}#com-{com_id}')" class="comment-button" alt="Copy link"><i class="fa-solid fa-link"></i></a>"##,
		));
		if user_id == Some(&comment.author_id) || admin {
			buttons.push(format!(
				r##"<a href="#comment-{com_id}-edit-content" class="comment-button" onclick="toggle_edit('{com_id}')"><i class="fa-solid fa-pen-to-square"></i></a>"##
			));
			buttons.push(format!(
				r##"<a class="comment-button" onclick="del('{com_id}')"><i class="fa-solid fa-trash"></i></a>"##
			));
		}
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

	if comment.remove_date.is_some() && !admin {
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
	let author = User::from_id(db, &comment.author_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(author) = author else {
		return Ok(String::new());
	};
	let html_url = ammonia::clean(&author.github_html_url);
	let login = ammonia::clean(&author.github_login);

	// Translate markdown
	let markdown = util::markdown_to_html(&comment.content.content, true);

	let mut date_text = if comment.content.edit_date > comment.post_date {
		format!(
			r#"<span id="date-long">{}</span> (edit: <span id="date-long">{}</span>)"#,
			comment.post_date.and_utc().to_rfc3339(),
			comment.content.edit_date.and_utc().to_rfc3339()
		)
	} else {
		format!(
			r#"<span id="date-long">{}</span>"#,
			comment.post_date.and_utc().to_rfc3339()
		)
	};
	if comment.remove_date.is_some() && admin {
		date_text.push_str(" - REMOVED");
	}

	let (edit_editor, reply_editor) = match user_login {
		Some(user_login) => (
			get_editor(
				user_login,
				"edit",
				Some(com_id),
				Some(&comment.content.content),
			),
			get_editor(user_login, "post", Some(com_id), None),
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
