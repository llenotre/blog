use crate::service::article::Article;
use crate::service::comment;
use crate::service::comment::{Comment, CommentContent, MAX_CHARS};
use crate::service::user::User;
use crate::util::{now, Oid};
use crate::GlobalData;
use actix_session::Session;
use actix_web::{delete, error, get, patch, post, web, HttpResponse, Responder};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tracing::error;

/// Minimum post cooldown.
const INTERVAL: Duration = Duration::from_secs(10);

// TODO cleanup: avoid duplicate code and fix errors handling

#[get("/comment/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<Oid>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let id = id.into_inner();

	let user = User::current_user(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let admin = user.as_ref().map(|u| u.admin).unwrap_or(false);

	let comment = Comment::from_id(&data.db, &id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;
	if comment.remove_date.is_some() && !admin {
		return Ok(HttpResponse::NotFound()
			.content_type("text/plain")
			.body("comment not found"));
	}

	let article = Article::from_id(&data.db, &comment.article_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;

	// Get replies
	let replies = match comment.reply_to {
		None => Some(
			comment
				.get_replies(&data.db)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "mongodb");
					error::ErrorInternalServerError("")
				})?
				.collect::<Vec<_>>()
				.await,
		),
		Some(_) => None,
	};

	let user_id = user.as_ref().map(|u| &u.id);
	let user_login = user.as_ref().map(|u| u.github_login.as_str());
	let html = comment::to_html(
		&data.db,
		&article.content.title,
		&comment,
		replies.as_deref(),
		user_id,
		user_login,
		admin,
	)
	.await?;
	Ok(HttpResponse::Ok().body(html))
}

/// The payload for the request allowing to post a comment.
#[derive(Deserialize)]
pub struct PostCommentPayload {
	/// The ID of the article.
	article_id: Oid,
	/// The ID of the comment this comment responds to.
	///
	/// If `None`, this comment is not a response.
	reply_to: Option<Oid>,

	/// The content of the comment in markdown.
	content: String,
}

#[post("/comment")]
pub async fn post(
	data: web::Data<GlobalData>,
	info: web::Json<PostCommentPayload>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let info = info.into_inner();

	let len = info.content.as_bytes().len();
	if len == 0 {
		return Ok(HttpResponse::BadRequest()
			.content_type("text/plain")
			.body("comment is empty"));
	}
	if len > MAX_CHARS {
		return Ok(HttpResponse::BadRequest()
			.content_type("text/plain")
			.body(format!(
				"comment is too long ({len}/{MAX_CHARS} characters)"
			)));
	}

	// Check article exists
	let article = Article::from_id(&data.db, &info.article_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound("article not found"));
	};

	// Get user
	let user = User::current_user(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(user) = user else {
		return Ok(HttpResponse::Forbidden()
			.content_type("text/plain")
			.body("login first"));
	};

	if !user.admin {
		if !article.content.public {
			return Ok(HttpResponse::Forbidden()
				.content_type("text/plain")
				.body("article not found"));
		}
		if article.content.comments_locked {
			return Ok(HttpResponse::Forbidden()
				.content_type("text/plain")
				.body("comments are locked"));
		}

		// Check user's cooldown
		let now = now();
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(HttpResponse::TooManyRequests()
				.content_type("text/plain")
				.body(format!("wait {remaining} seconds before retrying")));
		}
	}

	let date = now();
	let comment_id = Comment::create(
		&data.db,
		&info.article_id,
		&info.reply_to,
		&user.id,
		&date,
		&info.content,
	)
	.await
	.map_err(|e| {
		error!(error = %e, "postgres: comment create");
		error::ErrorInternalServerError("")
	})?;

	Ok(HttpResponse::Ok().json(json!({
		"id": comment_id
	})))
}

/// The payload for the request allowing to edit a comment.
#[derive(Deserialize)]
pub struct EditCommentPayload {
	/// The ID of the comment.
	comment_id: Oid,
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
		return Err(error::ErrorBadRequest("no content provided"));
	}
	if info.content.as_bytes().len() > MAX_CHARS {
		return Err(error::ErrorPayloadTooLarge("content is too long"));
	}

	// Get comment
	let comment = Comment::from_id(&data.db, &info.comment_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(comment) = comment else {
		return Err(error::ErrorNotFound("comment not found"));
	};

	let article = Article::from_id(&data.db, &comment.article_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound("article not found"));
	};

	// Get user
	let user = User::current_user(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(user) = user else {
		return Err(error::ErrorForbidden("forbidden"));
	};

	let now = now();

	if !user.admin {
		if !article.content.public {
			return Err(error::ErrorNotFound("article not found"));
		}
		if article.content.comments_locked {
			return Ok(HttpResponse::Forbidden()
				.content_type("text/plain")
				.body("comments are locked"));
		}
		if comment.author_id != user.id {
			return Err(error::ErrorForbidden("forbidden"));
		}

		// Check user's cooldown
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(HttpResponse::TooManyRequests()
				.content_type("text/plain")
				.body(format!("wait {remaining} seconds before retrying")));
		}
	}

	// Insert comment content
	let comment_content = CommentContent {
		comment_id: info.comment_id,
		edit_date: now,
		content: info.content,
	};
	Comment::edit(&data.db, &user.id, &comment_content)
		.await
		.map_err(|e| {
			error!(error = %e, "postgres: comment edit");
			error::ErrorInternalServerError("")
		})?;

	Ok(HttpResponse::Ok().finish())
}

#[delete("/comment/{id}")]
pub async fn delete(
	data: web::Data<GlobalData>,
	comment_id: web::Path<Oid>,
	session: Session,
) -> impl Responder {
	let comment_id = comment_id.into_inner();

	let Some(user_id) = session.get::<Oid>("user_id").ok().flatten() else {
		return Err(error::ErrorForbidden("forbidden"));
	};

	// Delete if the user has permission
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	Comment::delete(&data.db, &comment_id, &user_id, admin)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// TODO change status according to error (not found, forbidden, etc...)
	Ok(HttpResponse::Ok().finish())
}
