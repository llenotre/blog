mod middleware;
mod route;
mod service;
mod util;

use crate::middleware::analytics::Analytics;
use crate::service::analytics::AnalyticsEntry;
use actix_files::Files;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::middleware::Logger;
use actix_web::{
	body::BoxBody, body::EitherBody, cookie::Key, dev::ServiceResponse, http::header,
	http::header::HeaderValue, middleware::ErrorHandlerResponse, middleware::ErrorHandlers, web,
	App, HttpServer,
};
use base64::Engine;
use mongodb::options::ClientOptions;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::process::exit;
use std::time::Duration;
use tokio::time;

/// Server configuration.
#[derive(Deserialize)]
struct Config {
	/// The HTTP server's port.
	port: u16,
	/// The URL to the mongodb database.
	mongo_url: String,

	/// The client ID of the Github application.
	client_id: String,
	/// The client secret of the Github application.
	client_secret: String,

	/// The secret key used to secure sessions.
	session_secret_key: String,

	/// The URL to the Discord server's invitation.
	discord_invite: String,
}

/// Structure shared across the server.
pub struct GlobalData {
	/// The connection to the MongoDB database.
	pub mongo: mongodb::Client,

	/// The client ID of the Github application.
	pub client_id: String,
	/// The client secret of the Github application.
	pub client_secret: String,

	/// The URL to the Discord server's invitation.
	pub discord_invite: String,
}

impl GlobalData {
	/// Returns a reference to the database.
	pub fn get_database(&self) -> mongodb::Database {
		self.mongo.database("blog")
	}
}

fn error_handler<B>(res: ServiceResponse<B>) -> actix_web::Result<ErrorHandlerResponse<B>> {
	let pretty_error = res
		.headers()
		.get("Content-Type")
		.map(HeaderValue::to_str)
		.transpose()
		.unwrap()
		.map(|s| s != "text/plain" && s != "application/json")
		.unwrap_or(true);
	let response = if pretty_error {
		let html = include_str!("../pages/error.html");
		let status = res.status();
		let html = html.replace("{error.code}", &status.as_u16().to_string());
		let html = html.replace("{error.reason}", status.canonical_reason().unwrap());

		let (req, res) = res.into_parts();
		let res = res.map_body(|_, _| EitherBody::Right {
			body: BoxBody::new(html),
		});

		let mut response = ServiceResponse::new(req, res);
		response
			.response_mut()
			.headers_mut()
			.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
		response
	} else {
		res.map_body(|_, body| EitherBody::Left { body })
	};
	Ok(ErrorHandlerResponse::Response(response))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
	// Enabling logging
	env::set_var("RUST_LOG", "info");
	env_logger::init();

	// Read configuration
	let config = fs::read_to_string("config.toml")
		.map(|s| toml::from_str::<Config>(&s))
		.unwrap_or_else(|e| {
			tracing::error!(error = %e, "Cannot open configuration file");
			exit(1);
		})
		.unwrap_or_else(|e| {
			tracing::error!(error = %e, "invalid configuration file");
			exit(1);
		});

	// Open database connection
	let client_options = ClientOptions::parse(&config.mongo_url)
		.await
		.unwrap_or_else(|e| {
			tracing::error!(error = %e, "mongodb");
			exit(1);
		});
	let client = mongodb::Client::with_options(client_options).unwrap_or_else(|e| {
		tracing::error!(error = %e, "mongodb");
		exit(1);
	});

	let data = web::Data::new(GlobalData {
		mongo: client,

		client_id: config.client_id,
		client_secret: config.client_secret,

		discord_invite: config.discord_invite,
	});

	let session_secret_key = base64::engine::general_purpose::STANDARD
		.decode(config.session_secret_key)
		.unwrap();

	// Worker task
	let data_clone = data.clone();
	tokio::spawn(async move {
		let data = data_clone.into_inner();
		let db = data.get_database();
		let mut interval = time::interval(Duration::from_secs(10));

		loop {
			let _ = AnalyticsEntry::aggregate(&db).await;
			interval.tick().await;
		}
	});

	HttpServer::new(move || {
		App::new()
			.service(Files::new("/assets", "./assets"))
			.service(route::article::editor)
			.service(route::article::get)
			.service(route::article::post)
			.service(route::comment::get)
			.service(route::bio)
			.service(route::comment::delete)
			.service(route::comment::edit)
			.service(route::comment::post)
			.service(route::file::delete)
			.service(route::file::get)
			.service(route::file::manage)
			.service(route::file::upload)
			.service(route::legal)
			.service(route::newsletter::subscribe)
			.service(route::robots)
			.service(route::root)
			.service(route::rss)
			.service(route::sitemap)
			.service(route::user::auth)
			.service(route::user::avatar)
			.service(route::user::logout)
			.service(route::user::oauth)
			.wrap(ErrorHandlers::new().default_handler(error_handler))
			.wrap(Analytics {
				global: data.clone().into_inner(),
			})
			.wrap(SessionMiddleware::new(
				CookieSessionStore::default(),
				Key::from(&session_secret_key),
			))
			.app_data(data.clone())
			.app_data(web::PayloadConfig::new(1024 * 1024))
			.wrap(Logger::new("[%t] %a: %r - Response: %s (in %D ms)"))
	})
	.bind(format!("0.0.0.0:{}", config.port))?
	.run()
	.await
}
