use crate::service::article::{Article, ArticleContent};
use crate::service::comment::Comment;
use crate::service::user::User;
use crate::service::{comment, user};
use crate::util::{now, Oid};
use crate::{util, GlobalData};
use actix_session::Session;
use actix_web::http::header::ContentType;
use actix_web::web::Redirect;
use actix_web::{error, get, post, web, Either, HttpResponse, Responder};
use futures_util::StreamExt;
use serde::Deserialize;
use tracing::error;

#[get("/a/{id}/{title}")]
pub async fn get(
	data: web::Data<GlobalData>,
	path: web::Path<(Oid, String)>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let (id, title) = path.into_inner();

	// Get article
	let article = Article::from_id(&data.db, &id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let Some(article) = article else {
		return Err(error::ErrorNotFound(""));
	};

	// If URL title does not match, redirect
	let expected_title = article.content.get_url_title();
	if title != expected_title {
		return Ok(Either::Left(
			Redirect::to(article.get_path()).see_other(),
		));
	}

	// If article is not public, the user must be admin to see it
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if (!article.content.public || article.post_date.is_none()) && !admin {
		return Err(error::ErrorNotFound(""));
	}
	let post_date = if let Some(post_date) = article.post_date {
		post_date.and_utc().to_rfc3339()
	} else {
		"not posted yet".to_string()
	};

	let html = include_str!("../../pages/article.html");
	let html = html.replace("{article.tags}", &article.content.tags);
	let html = html.replace("{article.id}", &id.to_string());
	let html = html.replace("{article.url}", &article.get_url());
	let html = html.replace("{article.title}", &article.content.title);
	let html = html.replace("{article.date}", &post_date);
	let html = html.replace("{article.description}", &article.content.description);
	let html = html.replace("{article.cover_url}", &article.content.cover_url);
	let markdown = util::markdown_to_html(&article.content.content, false);
	let html = html.replace("{article.content}", &markdown);

	let user_id = session.get::<Oid>("user_id")?;
	let user_login = session.get::<String>("user_login")?;

	// Get article comments
	let comments = Comment::list_for_article(&data.db, &article.id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?
		.collect::<Vec<_>>()
		.await;
	let comments_count = comments.len();
	let html = html.replace("{comments.count}", &comments_count.to_string());

	let comments = comment::group(comments);
	let mut comments_html = String::new();
	for (com, replies) in comments {
		if !admin && com.remove_date.is_some() && replies.is_empty() {
			continue;
		}

		comments_html.push_str(
			&comment::to_html(
				&data.db,
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

	session.insert("last_article", id)?;
	Ok(Either::Right(
		HttpResponse::Ok()
			.content_type(ContentType::html())
			.body(html),
	))
}

/// Editor page query.
#[derive(Deserialize)]
pub struct EditorQuery {
	/// The ID of the article to edit.
	///
	/// If `None`, a new article is being created.
	id: Option<Oid>,
}

#[get("/editor")]
pub async fn editor(
	data: web::Data<GlobalData>,
	query: web::Query<EditorQuery>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	// Check auth
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorNotFound(""));
	}

	// Get article
	let article_id = query.into_inner().id;
	let article = match article_id {
		Some(article_id) => Article::from_id(&data.db, &article_id)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?,
		None => None,
	};

	let article_id_html = article
		.as_ref()
		.map(|a| {
			format!(
				"<input name=\"id\" type=\"hidden\" value=\"{article_id}\" />",
				article_id = a.id
			)
		})
		.unwrap_or_default();
	let article_title = article
		.as_ref()
		.map(|a| a.content.title.as_str())
		.unwrap_or("");
	let article_desc = article
		.as_ref()
		.map(|a| a.content.description.as_str())
		.unwrap_or("");
	let article_cover_url = article
		.as_ref()
		.map(|a| a.content.cover_url.as_str())
		.unwrap_or("");
	let article_content = article
		.as_ref()
		.map(|a| a.content.content.as_str())
		.unwrap_or("");
	let article_public = article.as_ref().map(|a| a.content.public).unwrap_or(false);
	let article_sponsor = article.as_ref().map(|a| a.content.sponsor).unwrap_or(false);
	let comments_locked = article
		.as_ref()
		.map(|a| a.content.comments_locked)
		.unwrap_or(false);
	let article_tags = article
		.as_ref()
		.map(|a| a.content.tags.as_str())
		.unwrap_or("");

	let html = include_str!("../../pages/editor.html");
	let html = html.replace("{article.id}", &article_id_html);
	let html = html.replace("{article.title}", article_title);
	let html = html.replace("{article.description}", article_desc);
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
	id: Option<Oid>,

	/// The title of the article.
	title: String,
	/// The description of the article.
	description: String,
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
	// Check auth
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

	let info = info.into_inner();
	let public = info.public.map(|p| p == "on").unwrap_or(false);
	let sponsor = info.sponsor.map(|p| p == "on").unwrap_or(false);
	let comments_locked = info.comments_locked.map(|p| p == "on").unwrap_or(false);

	let date = now();
	let path = match info.id {
		// Update article
		Some(id) => {
			// Insert article content
			let content = ArticleContent {
				title: info.title,
				description: info.description,
				cover_url: info.cover_url,
				content: info.content,
				tags: info.tags,
				public,
				sponsor,
				comments_locked,

				edit_date: date,
			};
			Article::edit(&data.db, &content, &date)
				.await
				.map_err(|e| {
					tracing::error!(error = %e, "mongodb");
					error::ErrorInternalServerError("")
				})?;

			content.get_path()
		}

		// Create article
		None => {
			let content = ArticleContent {
				edit_date: date,
				title: info.title,
				description: info.description,
				cover_url: info.cover_url,
				content: info.content,
				tags: info.tags,
				public,
				sponsor,
				comments_locked,
			};
			Article::create(&data.db, &content).await
				.map_err(|e| {
					error!(error = %e, "postgres: article insert");
					error::ErrorInternalServerError("")
				})?;

			content.get_path()
		}
	};

	Ok(Redirect::to(path).see_other())
}
