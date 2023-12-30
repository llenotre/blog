mod config;
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
use config::{Config, GithubConfig};
use std::env;
use std::fs;
use std::io;
use std::process::exit;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tokio_postgres::NoTls;
use tracing::{error, info};

/// Structure shared across the server.
pub struct GlobalData {
	/// The connection to the database.
	pub db: RwLock<tokio_postgres::Client>,

	/// Github configuration.
	pub github_config: GithubConfig,
	/// The URL to the Discord server's invitation.
	pub discord_invite: String,
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
		res.map_body(|_, body| EitherBody::Left {
			body,
		})
	};
	Ok(ErrorHandlerResponse::Response(response))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
	// Enable logging
	env::set_var("RUST_LOG", "info");
	env_logger::init();

	info!("read configuration");

	// Read configuration
	let config = fs::read_to_string("config.toml").unwrap_or_else(|error| {
		error!(%error, "cannot read configuration file");
		exit(1);
	});
	let config: Config = toml::from_str(&config).unwrap_or_else(|error| {
		error!(%error, "invalid configuration file");
		exit(1);
	});
	let session_secret_key = base64::engine::general_purpose::STANDARD
		.decode(config.session_secret_key)
		.unwrap();

	info!("connect to database");

	// Open database connection
	// TODO tls
	let (client, connection) = tokio_postgres::connect(&config.db, NoTls)
		.await
		.unwrap_or_else(|error| {
			error!(%error, "postgres: connection");
			exit(1);
		});

	let data = web::Data::new(GlobalData {
		db: RwLock::new(client),

		github_config: config.github,
		discord_invite: config.discord_invite,
	});

	// Handle connection errors
	let data_clone = data.clone();
	tokio::spawn(async move {
		let data = data_clone;
		let mut connection = connection;
		loop {
			// Wait for the connection to close
			if let Err(error) = connection.await {
				error!(%error, "postgres: connection");
			}

			// Try to reconnect
			let mut interval = time::interval(Duration::from_secs(10));
			loop {
				interval.tick().await;

				info!("postgres: attempting to reconnect");
				// TODO tls
				let res = tokio_postgres::connect(&config.db, NoTls).await;
				match res {
					Ok((client, c)) => {
						info!("postgres: reconnected successfully");
						*data.db.write().await = client;
						connection = c;
						break;
					}
					Err(error) => {
						error!(%error, "postgres: connection");
						continue;
					}
				}
			}
		}
	});

	info!("start worker");

	// Worker task
	let data_clone = data.clone();
	tokio::spawn(async move {
		let data = data_clone.into_inner();
		let mut interval = time::interval(Duration::from_secs(10));
		loop {
			let _ = AnalyticsEntry::aggregate(&*data.db.read().await).await;
			interval.tick().await;
		}
	});

	info!("start http server");

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
			.service(route::legal)
			.service(route::newsletter::subscribe)
			.service(route::newsletter::unsubscribe)
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
