use crate::service::user::User;
use crate::GlobalData;
use actix_session::Session;
use actix_web::http::header::ContentType;
use actix_web::{error, get, web, HttpResponse, Responder};
use tracing::error;

#[get("/a/{url_title}")]
pub async fn get(
	data: web::Data<GlobalData>,
	path: web::Path<String>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let url_title = path.into_inner();

	// Get article
	let article = data.get_article(&url_title);
	let Some((article, content)) = article else {
		return Err(error::ErrorNotFound(""));
	};

	// If article is not public, the user must be admin to see it
	let admin = {
		let db = data.db.read().await;
		User::check_admin(&db, &session).await.map_err(|e| {
			error!(error = %e, "postgres: check admin");
			error::ErrorInternalServerError("")
		})?
	};
	if !article.public && !admin {
		return Err(error::ErrorNotFound(""));
	}

	let tags: String = article
		.tags
		.iter()
		.map(|s| s.as_ref())
		.intersperse(",")
		.collect();
	let post_date = article.post_date.to_rfc3339();

	let html = include_str!("../../pages/article.html");
	let html = html.replace("{article.tags}", &tags);
	let html = html.replace("{article.url}", &article.get_url());
	let html = html.replace("{article.title}", &article.title);
	let html = html.replace("{article.date}", &post_date);
	let html = html.replace("{article.description}", &article.description);
	let html = html.replace("{article.cover_url}", &article.cover_url);
	let html = html.replace("{article.content}", &content);

	session.insert("last_article", url_title)?;
	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}
