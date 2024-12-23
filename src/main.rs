mod config;
mod route;
mod service;

use crate::service::article::Article;
use axum::{
	extract::State,
	http::StatusCode,
	response::{Html, IntoResponse, Redirect, Response},
	routing::get,
	Router,
};
use config::Config;
use gateway_api::log::LogLayer;
use std::{collections::HashMap, io, net::SocketAddr, process::exit, sync::Arc};
use tower_http::services::ServeDir;
use tracing::{error, info};

/// Structure shared across the server.
pub struct Context {
	/// Configuration of the gateway API.
	pub gateway_config: &'static gateway_api::Config,

	/// The URL to the Discord server's invitation.
	pub discord_invite: String,
	/// Articles along with their respective compiled content, ordered by post date.
	pub articles: Vec<(Article, String)>,
	/// A map to find an article index from its slug.
	pub articles_index: HashMap<String, usize>,
}

impl Context {
	/// Returns the article and compiled content with the given slug.
	pub fn get_article(&self, slug: &str) -> Option<&(Article, String)> {
		let index = *self.articles_index.get(slug)?;
		Some(&self.articles[index])
	}

	/// Returns the list of articles without their content.
	pub fn list_articles(&self) -> impl Iterator<Item = &Article> {
		self.articles.iter().map(|(a, _)| a)
	}
}

async fn handle_404() -> Response {
	let html = include_str!("../pages/error.html");
	let status = StatusCode::NOT_FOUND;
	let html = html.replace("{error.code}", &status.as_u16().to_string());
	let html = html.replace("{error.reason}", status.canonical_reason().unwrap());
	(status, Html(html)).into_response()
}

#[tokio::main]
async fn main() -> io::Result<()> {
	tracing_subscriber::fmt::init();
	let config = envy::prefixed("BLOG_")
		.from_env::<Config>()
		.unwrap_or_else(|error| {
			error!(%error, "invalid configuration");
			exit(1);
		});
	info!("compile all articles");
	let articles = Article::compile_all(&config.article_path).unwrap_or_else(|error| {
		error!(%error, "could not compile articles");
		exit(1);
	});
	let articles_index = articles
		.iter()
		.enumerate()
		.map(|(i, (a, _))| (a.slug.clone(), i))
		.collect();
	info!("{} articles found", articles.len());
	let ctx = Arc::new(Context {
		gateway_config: gateway_api::Config::get(),

		discord_invite: config.discord_invite,
		articles,
		articles_index,
	});
	info!("start http server");
	let router = Router::new()
		.nest_service("/assets", ServeDir::new("assets"))
		.nest_service("/assets/article", ServeDir::new(config.article_assets_path))
		// deprecated route
		.route(
			"/avatar/llenotre",
			get(|State(ctx): State<Arc<Context>>| async move {
				let url = format!("{}/avatar", ctx.gateway_config.gateway_url);
				Redirect::permanent(&url)
			}),
		)
		.route("/health", get(route::health))
		.route("/", get(route::root))
		.route("/a/:slug", get(route::article::get))
		.route("/bio", get(route::bio))
		.route("/legal", get(route::legal))
		.route("/robots.txt", get(gateway_api::robots))
		.route("/sitemap.xml", get(route::sitemap))
		.route("/rss", get(route::rss))
		.fallback(handle_404);
	#[cfg(feature = "analytics")]
	let router = router.layer(gateway_api::analytics::AnalyticsLayer::default());
	let router = router
		.layer(LogLayer)
		.with_state(ctx.clone())
		.into_make_service_with_connect_info::<SocketAddr>();
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
	axum::serve(listener, router).await
}
