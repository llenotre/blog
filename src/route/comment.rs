use crate::service::article::Article;
use crate::service::comment;
use crate::service::comment::{Comment, CommentContent, MAX_CHARS};
use crate::service::user::User;
use crate::{util, GlobalData};
use actix_session::Session;
use actix_web::{delete, error, get, patch, post, web, HttpResponse, Responder};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

/// Minimum post cooldown.
const INTERVAL: Duration = Duration::from_secs(10);

// TODO cleanup: avoid duplicate code and fix errors handling

#[get("/comment/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let id = id.into_inner();
	let db = data.get_database();

	let user = User::current_user(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let comment_id = util::decode_id(&id).ok_or_else(|| error::ErrorNotFound(""))?;
	let comment = Comment::from_id(&db, &comment_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;
	let admin = user.as_ref().map(|u| u.admin).unwrap_or(false);
	if comment.removed && !admin {
		return Ok(HttpResponse::NotFound()
			.content_type("text/plain")
			.body("comment not found"));
	}

	let article = Article::from_id(&db, &comment.article)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?
		.ok_or_else(|| error::ErrorNotFound("comment not found"))?;

	// Get replies
	let replies = match comment.reply_to {
		None => Some(comment.get_replies(&db).await.map_err(|e| {
			tracing::error!(error = %e, "mongodb");
			error::ErrorInternalServerError("")
		})?),
		Some(_) => None,
	};

	let user_id = user.as_ref().map(|u| &u.id);
	let user_login = user.as_ref().map(|u| u.github_info.login.as_str());
	let html = comment::to_html(
		&db,
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
	article_id: String,
	/// The ID of the comment this comment responds to. If `None`, this comment is not a response.
	reply_to: Option<String>,

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

	let db = data.get_database();

	// Check article exists
	let article_id = util::decode_id(&info.article_id).ok_or_else(|| error::ErrorNotFound(""))?;
	let article = Article::from_id(&db, &article_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound("article not found"));
	};

	// Get user
	let user = User::current_user(&db, &session)
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
		let now = Utc::now();
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(HttpResponse::TooManyRequests()
				.content_type("text/plain")
				.body(format!("wait {remaining} seconds before retrying")));
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

	// unwrap cannot fail because getting the article already decodes the ID
	let article_id = util::decode_id(&info.article_id).unwrap();

	let reply_to = info
		.reply_to
		.as_deref()
		.map(util::decode_id)
		.map(|id| match id {
			Some(id) => Ok(id),
			None => Err(HttpResponse::NotFound()
				.content_type("text/plain")
				.body("comments not found")),
		})
		.transpose();
	let reply_to = match reply_to {
		Ok(id) => id,
		Err(e) => return Ok(e),
	};

	let comment = Comment {
		id,

		article: article_id,
		reply_to,
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
		"id": util::encode_id(&comment.id)
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

	// Check comment exists
	let comment_id = util::decode_id(&info.comment_id).ok_or_else(|| error::ErrorNotFound(""))?;
	let comment = Comment::from_id(&db, &comment_id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(comment) = comment else {
		return Err(error::ErrorNotFound("comment not found"));
	};

	let article = Article::from_id(&db, &comment.article)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound("article not found"));
	};

	// Get user
	let user = User::current_user(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(user) = user else {
		return Err(error::ErrorForbidden("forbidden"));
	};

	if !user.admin {
		if !article.content.public {
			return Err(error::ErrorNotFound("article not found"));
		}
		if article.content.comments_locked {
			return Ok(HttpResponse::Forbidden()
				.content_type("text/plain")
				.body("comments are locked"));
		}
		if comment.author != user.id {
			return Err(error::ErrorForbidden("forbidden"));
		}

		// Check user's cooldown
		let now = Utc::now();
		let cooldown_end = user.last_post + chrono::Duration::from_std(INTERVAL).unwrap();
		if now < cooldown_end {
			let remaining = (cooldown_end - now).num_seconds();
			return Ok(HttpResponse::TooManyRequests()
				.content_type("text/plain")
				.body(format!("wait {remaining} seconds before retrying")));
		}
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
	let comment_id =
		util::decode_id(&comment_id.into_inner()).ok_or_else(|| error::ErrorNotFound(""))?;

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
