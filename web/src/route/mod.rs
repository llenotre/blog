use crate::service::user::User;
use crate::GlobalData;
use actix_session::Session;
use actix_web::http::header::ContentType;
use actix_web::{error, get, web, HttpResponse, Responder};

pub mod article;
pub mod newsletter;
pub mod user;

#[get("/")]
pub async fn root(
	data: web::Data<GlobalData>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.db.read().await;
	let admin = User::check_admin(&db, &session).await.map_err(|e| {
		tracing::error!(error = %e, "database: user");
		error::ErrorInternalServerError("")
	})?;

	// Get articles
	let articles: String = data
		.list_articles()
		.filter(|a| a.is_public() || admin)
		.map(|a| a.display_list_html(admin).to_string())
		.collect();

	let html = include_str!("../../pages/index.html");
	let html = html.replace("{discord}", &data.discord_invite);
	let html = html.replace("{articles}", &articles);
	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}

#[get("/bio")]
pub async fn bio() -> impl Responder {
	let html = include_str!("../../pages/bio.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/legal")]
pub async fn legal() -> impl Responder {
	let html = include_str!("../../pages/legal.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/robots.txt")]
pub async fn robots() -> impl Responder {
	r#"User-agent: *
Allow: /
Sitemap: https://blog.lenot.re/sitemap.xml"#
}

#[get("/sitemap.xml")]
pub async fn sitemap(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let articles: String = data
		.list_articles()
		.filter(|a| a.is_public())
		.map(|a| a.display_sitemap().to_string())
		.collect();
	let body = format!(
		r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
	<url><loc>/</loc></url>
	<url><loc>/bio</loc></url>
	<url><loc>/legal</loc></url>
	{articles}
</urlset>"#
	);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::xml())
		.body(body))
}

#[get("/rss")]
pub async fn rss(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let articles: String = data
		.list_articles()
		.filter(|a| a.is_public())
		.map(|a| a.display_rss().to_string())
		.collect();
	let body = format!(
		r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel><atom:link href="https://blog.lenot.re/rss" rel="self" type="application/rss+xml" /><title>Maestro</title><link>https:/blog.lenot.re/</link><description>A blog about writing an operating system from scratch in Rust.</description>{articles}</channel></rss>"#
	);
	Ok(HttpResponse::Ok()
		.content_type("application/rss+xml")
		.body(body))
}
