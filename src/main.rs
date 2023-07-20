mod article;
mod comment;
mod middleware;
mod newsletter;
mod route;
mod user;
mod util;

use crate::middleware::analytics::Analytics;
use actix_files::Files;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::middleware::Logger;
use actix_web::{
	App, body::BoxBody, body::EitherBody, cookie::Key, dev::ServiceResponse,
	http::header, http::header::HeaderValue,
	HttpServer, middleware::ErrorHandlerResponse, middleware::ErrorHandlers, web,
};
use base64::Engine;
use mongodb::options::ClientOptions;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::process::exit;

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
	let status = res.status();

	let html = include_str!("../pages/error.html");
	let html = html.replace("{error.code}", &format!("{}", status.as_u16()));
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
			eprintln!("Cannot open configuration file: {}", e);
			exit(1);
		})
		.unwrap_or_else(|e| {
			eprintln!("Invalid configuration file: {}", e);
			exit(1);
		});

	// Open database connection
	let client_options = ClientOptions::parse(&config.mongo_url)
		.await
		.unwrap_or_else(|e| {
			eprintln!("mongodb: {e}");
			exit(1);
		});
	let client = mongodb::Client::with_options(client_options).unwrap_or_else(|e| {
		eprintln!("mongodb: {e}");
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
			.service(newsletter::subscribe)
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
