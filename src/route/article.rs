use crate::service::article::{Article, ArticleContent};
use crate::service::comment::Comment;
use crate::service::user::User;
use crate::service::{comment, user};
use crate::{util, GlobalData};
use actix_session::Session;
use actix_web::http::header::ContentType;
use actix_web::web::Redirect;
use actix_web::{error, get, post, web, Either, HttpResponse, Responder};
use chrono::Utc;
use serde::Deserialize;

#[get("/a/{id}/{title}")]
pub async fn get(
	data: web::Data<GlobalData>,
	path: web::Path<(String, String)>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();

	let (id_str, title) = path.into_inner();
	let id = util::decode_id(&id_str).ok_or_else(|| error::ErrorNotFound(""))?;

	// Get article
	let article = Article::from_id(&db, &id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound(""));
	};
	let content = article
		.get_content(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// If URL title does not match, redirect
	let expected_title = content.get_url_title();
	if title != expected_title {
		return Ok(Either::Left(Redirect::to(content.get_path()).see_other()));
	}

	// If article is not public, the user must be admin to see it
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if (!content.public || article.post_date.is_none()) && !admin {
		return Err(error::ErrorNotFound(""));
	}
	let post_date = if let Some(post_date) = article.post_date {
		post_date.to_rfc3339()
	} else {
		"not posted yet".to_string()
	};

	let html = include_str!("../../pages/article.html");
	let html = html.replace("{article.tags}", &content.tags);
	let html = html.replace("{article.id}", &id_str);
	let html = html.replace("{article.url}", &content.get_url());
	let html = html.replace("{article.title}", &content.title);
	let html = html.replace("{article.date}", &post_date);
	let html = html.replace("{article.desc}", &content.desc);
	let html = html.replace("{article.cover_url}", &content.cover_url);
	let markdown = util::markdown_to_html(&content.content, false);
	let html = html.replace("{article.content}", &markdown);

	let user_id = session
		.get::<String>("user_id")?
		.map(|id| ObjectId::parse_str(id).map_err(|_| error::ErrorBadRequest("")))
		.transpose()?;
	let user_login = session.get::<String>("user_login")?;

	// Get article comments
	let comments = Comment::list_for_article(&db, article.id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let comments_count = comments.len();
	let html = html.replace("{comments.count}", &comments_count.to_string());

	let comments = comment::group(comments);
	let mut comments_html = String::new();
	for (com, replies) in comments {
		if !admin && com.removed && replies.is_empty() {
			continue;
		}

		comments_html.push_str(
			&comment::to_html(
				&db,
				&expected_title,
				&com,
				Some(&replies),
				user_id.as_ref(),
				user_login.as_deref(),
				admin,
			)
			.await?,
		);
	}

	let html = html.replace("{comments}", &comments_html);

	let comment_editor_html = match user_login {
		Some(user_login) => comment::get_editor(&user_login, "post", None, None),

		None => format!(
			r#"<center>
                <a class="login-button" href="{}"><i class="fa-brands fa-github"></i>&nbsp;&nbsp;&nbsp;Sign in with Github to comment</a>
                <h6>By clicking, you accept the <a href="/legal#privacy" target="_blank">Privacy Policy</a></h6>
            </center>"#,
			user::get_auth_url(&data.client_id)
		),
	};
	let html = html.replace("{comment.editor}", &comment_editor_html);

	session.insert("last_article", id_str)?;
	Ok(Either::Right(
		HttpResponse::Ok()
			.content_type(ContentType::html())
			.body(html),
	))
}

/// Editor page query.
#[derive(Deserialize)]
pub struct EditorQuery {
	/// The ID of the article to edit. If `None`, a new article is being created.
	id: Option<String>,
}

#[get("/editor")]
pub async fn editor(
	data: web::Data<GlobalData>,
	query: web::Query<EditorQuery>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorNotFound(""));
	}

	// Get article
	let article_id = query
		.into_inner()
		.id
		.map(|id| util::decode_id(&id).ok_or_else(|| error::ErrorBadRequest("")))
		.transpose()?;
	let article = match article_id {
		Some(article_id) => Article::from_id(&db, &article_id)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?,
		None => None,
	};
	let content = match article.as_ref() {
		Some(article) => Some(
			article
				.get_content(&db)
				.await
				.map_err(|_| error::ErrorInternalServerError(""))?,
		),
		None => None,
	};

	let article_id_html = article
		.as_ref()
		.map(|a| {
			format!(
				"<input name=\"id\" type=\"hidden\" value=\"{article_id}\" />",
				article_id = util::encode_id(&a.id)
			)
		})
		.unwrap_or_default();
	let article_title = content.as_ref().map(|a| a.title.as_str()).unwrap_or("");
	let article_desc = content.as_ref().map(|a| a.desc.as_str()).unwrap_or("");
	let article_cover_url = content.as_ref().map(|a| a.cover_url.as_str()).unwrap_or("");
	let article_content = content.as_ref().map(|a| a.content.as_str()).unwrap_or("");
	let article_public = content.as_ref().map(|a| a.public).unwrap_or(false);
	let article_sponsor = content.as_ref().map(|a| a.sponsor).unwrap_or(false);
	let comments_locked = content.as_ref().map(|a| a.comments_locked).unwrap_or(false);
	let article_tags = content.as_ref().map(|a| a.tags.as_str()).unwrap_or("");

	let html = include_str!("../../pages/editor.html");
	let html = html.replace("{article.id}", &article_id_html);
	let html = html.replace("{article.title}", article_title);
	let html = html.replace("{article.desc}", article_desc);
	let html = html.replace("{article.cover_url}", article_cover_url);
	let html = html.replace("{article.content}", article_content);
	let html = html.replace(
		"{article.published}",
		if article_public { "checked" } else { "" },
	);
	let html = html.replace(
		"{article.sponsor}",
		if article_sponsor { "checked" } else { "" },
	);
	let html = html.replace(
		"{article.comments_locked}",
		if comments_locked { "checked" } else { "" },
	);
	let html = html.replace("{article.tags}", article_tags);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}

/// Article edition coming from the editor.
#[derive(Deserialize)]
pub struct ArticleEdit {
	/// The ID of the article. If `None`, a new article is being created.
	id: Option<String>,

	/// The title of the article.
	title: String,
	/// The description of the article.
	desc: String,
	/// The URL to the cover image of the article.
	cover_url: String,
	/// The content of the article in markdown.
	content: String,
	/// The comma-separated list of tags.
	tags: String,
	/// Tells whether to publish the article.
	public: Option<String>,
	/// Tells whether the article is reserved for sponsors.
	sponsor: Option<String>,
	/// Tells whether comments are locked on the article.
	comments_locked: Option<String>,
}

#[post("/a")]
pub async fn post(
	data: web::Data<GlobalData>,
	info: web::Form<ArticleEdit>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

	let info = info.into_inner();
	let public = info.public.map(|p| p == "on").unwrap_or(false);
	let sponsor = info.sponsor.map(|p| p == "on").unwrap_or(false);
	let comments_locked = info.comments_locked.map(|p| p == "on").unwrap_or(false);

	let date = Utc::now();
	let post_date = public.then_some(date);

	let path = match info.id {
		// Update article
		Some(id) => {
			let id = util::decode_id(&id).ok_or_else(|| error::ErrorNotFound(""))?;

			// Insert article content
			let content = ArticleContent {
				article_id: id,

				title: info.title,
				desc: info.desc,
				cover_url: info.cover_url,
				content: info.content,
				tags: info.tags,
				public,
				sponsor,
				comments_locked,

				edit_date: date,
			};
			let content_id = content.insert(&db).await.map_err(|e| {
				tracing::error!(error = %e, "mongodb");
				error::ErrorInternalServerError("")
			})?;

			Article::update(&db, id, content_id, post_date)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "mongodb");
					error::ErrorInternalServerError("")
				})?;

			content.get_path()
		}

		// Create article
		None => {
			let article_id = ObjectId::new();

			// Insert article content
			let content = ArticleContent {
				article_id,

				title: info.title,
				desc: info.desc,
				cover_url: info.cover_url,
				content: info.content,
				tags: info.tags,
				public,
				sponsor,
				comments_locked,

				edit_date: date,
			};
			let content_id = content.insert(&db).await.map_err(|e| {
				tracing::error!(error = %e, "mongodb");
				error::ErrorInternalServerError("")
			})?;

			let a = Article {
				id: article_id,
				content_id,
				post_date,
			};
			a.insert(&db).await.map_err(|e| {
				tracing::error!(error = %e, "mongodb");
				error::ErrorInternalServerError("")
			})?;

			content.get_path()
		}
	};

	Ok(Redirect::to(path).see_other())
}
