use crate::{
	Context,
	service::article::{ArticleListHtml, ArticleRss, ArticleSitemap},
};
use axum::{
	extract::State,
	http::header::CONTENT_TYPE,
	response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

pub mod article;

pub async fn health() -> &'static str {
	"OK"
}

pub async fn root(State(ctx): State<Arc<Context>>) -> Response {
	let articles: String = ctx
		.list_articles()
		.filter(|a| a.is_public())
		.map(|a| ArticleListHtml(a).to_string())
		.collect();
	let html = include_str!("../../pages/index.html");
	let html = html.replace("{discord}", &ctx.discord_invite);
	let html = html.replace("{gateway}", &ctx.gateway_config.gateway_url);
	let html = html.replace("{articles}", &articles);
	Html(html).into_response()
}

pub async fn bio() -> Response {
	Html(include_str!("../../pages/bio.html")).into_response()
}

pub async fn legal() -> Response {
	Html(include_str!("../../pages/legal.html")).into_response()
}

pub async fn sitemap(State(ctx): State<Arc<Context>>) -> Response {
	let articles: String = ctx
		.list_articles()
		.filter(|a| a.is_public())
		.map(|a| ArticleSitemap(a).to_string())
		.collect();
	let body = format!(
		r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
	<url><loc>https://blog.lenot.re/</loc></url>
	<url><loc>https://blog.lenot.re/bio</loc></url>
	<url><loc>https://blog.lenot.re/legal</loc></url>
{articles}
</urlset>"#
	);
	([(CONTENT_TYPE, "application/xml")], body).into_response()
}

pub async fn rss(State(ctx): State<Arc<Context>>) -> Response {
	let articles: String = ctx
		.list_articles()
		.filter(|a| a.is_public())
		.map(|a| ArticleRss(a).to_string())
		.collect();
	let body = format!(
		r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel><atom:link href="https://blog.lenot.re/rss" rel="self" type="application/rss+xml" /><title>Maestro</title><link>https:/blog.lenot.re/</link><description>A blog about writing an operating system from scratch in Rust.</description>{articles}</channel></rss>"#
	);
	([(CONTENT_TYPE, "application/rss+xml")], body).into_response()
}
