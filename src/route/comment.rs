use crate::article::Article;
use crate::comment::{comment_to_html, Comment, CommentContent, MAX_CHARS};
use crate::user::User;
use crate::GlobalData;
use actix_session::Session;
use actix_web::{delete, error, get, patch, post, web, HttpResponse, Responder};
use bson::oid::ObjectId;
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;
use serde_json::json;

/// Minimum post cooldown.
const INTERVAL: Duration = Duration::from_secs(10);

#[get("/comment/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let id = id.into_inner();
	let id = ObjectId::parse_str(id).map_err(|_| error::ErrorNotFound(""))?;

	let db = data.get_database();

	let user = User::current_user(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let comment = Comment::from_id(&db, &id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;
	let admin = user.as_ref().map(|u| u.admin).unwrap_or(false);
	if comment.removed && !admin {
		return Err(error::ErrorNotFound("comment not found"));
	}

	let article = Article::from_id(&db, &comment.article)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;
	let content = article.get_content(&db).await.map_err(|e| {
		tracing::error!(error = %e, "mongodb");
		error::ErrorInternalServerError("")
	})?;

	let user_id = user.as_ref().map(|u| &u.id);
	let user_login = user.as_ref().map(|u| u.github_info.login.as_str());
	let html = comment_to_html(
		&db,
		&content.title,
		&comment,
		None,
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
		return Err(error::ErrorBadRequest("no content provided"));
	}
	if info.content.as_bytes().len() > MAX_CHARS {
		return Err(error::ErrorPayloadTooLarge("content is too long"));
	}

	let db = data.get_database();

	// Check article exists
	let article_id = ObjectId::parse_str(info.article_id).map_err(|_| error::ErrorNotFound(""))?;
	let article = Article::from_id(&db, &article_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound("article not found"));
	};
	let article_content = article
		.get_content(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Get user
	let user = User::current_user(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(user) = user else {
		return Err(error::ErrorForbidden("forbidden"));
	};

	if !article_content.public && !user.admin {
		return Err(error::ErrorNotFound("article not found"));
	}

	// Check user's cooldown
	if !user.admin {
		let now = Utc::now();
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(
				HttpResponse::TooManyRequests().body(format!("wait {remaining} before retrying"))
			);
		}
	}

	let id = ObjectId::new();
	let date = Utc::now();

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
		author: user.id,
		post_date: date,

		content_id,

		removed: false,
	};
	comment
		.insert(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	user.update_cooldown(&db, Utc::now())
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	Ok(HttpResponse::Ok().json(json!({
		"id": comment.id.to_string()
	})))
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
		return Err(error::ErrorBadRequest("no content provided"));
	}
	if info.content.as_bytes().len() > MAX_CHARS {
		return Err(error::ErrorPayloadTooLarge("content is too long"));
	}

	let db = data.get_database();

	// Get user
	let user = User::current_user(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(user) = user else {
		return Err(error::ErrorForbidden("forbidden"));
	};

	// Check user's cooldown
	if !user.admin {
		let now = Utc::now();
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(
				HttpResponse::TooManyRequests().body(format!("wait {remaining} before retrying"))
			);
		}
	}

	// Check comment exists
	let comment_id = ObjectId::parse_str(info.comment_id).map_err(|_| error::ErrorNotFound(""))?;
	let comment = Comment::from_id(&db, &comment_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(comment) = comment else {
		return Err(error::ErrorNotFound("comment not found"));
	};

	if !user.admin && comment.author != user.id {
		return Err(error::ErrorForbidden("forbidden"));
	}

	// Insert comment content
	let date = Utc::now();
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

	user.update_cooldown(&db, Utc::now())
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
		return Err(error::ErrorForbidden("forbidden"));
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
