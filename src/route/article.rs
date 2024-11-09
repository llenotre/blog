use crate::Context;
use axum::{
	body::Body,
	extract::{Path, State},
	http::StatusCode,
	response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

pub async fn get(State(ctx): State<Arc<Context>>, Path(slug): Path<String>) -> Response {
	let Some((article, content)) = ctx.get_article(&slug) else {
		return (StatusCode::NOT_FOUND, Body::empty()).into_response();
	};
	if !article.is_public() {
		return StatusCode::NOT_FOUND.into_response();
	}
	let tags: String = article
		.tags
		.iter()
		.map(|s| s.as_ref())
		.fold(String::new(), |n1, n2: &str| n1 + "," + n2);
	let post_date = article.post_date.to_rfc3339();
	let html = include_str!("../../pages/article.html");
	let html = html.replace("{article.tags}", &tags);
	let html = html.replace("{article.url}", &article.get_url());
	let html = html.replace("{article.title}", &article.title);
	let html = html.replace("{article.date}", &post_date);
	let html = html.replace("{article.description}", &article.description);
	let html = html.replace("{article.cover_url}", &article.cover_url);
	let html = html.replace("{article.content}", &content);
	let html = html.replace("{discord}", &ctx.discord_invite);
	Html(html).into_response()
}
