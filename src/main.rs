mod article;
mod comment;
mod middleware;
mod newsletter;
mod route;
mod user;
mod util;

use crate::middleware::analytics::Analytics;
use crate::user::User;
use actix_files::Files;
use actix_session::storage::CookieSessionStore;
use actix_session::Session;
use actix_session::SessionMiddleware;
use actix_web::middleware::Logger;
use actix_web::{
	body::BoxBody, body::EitherBody, cookie::Key, dev::ServiceResponse, error, get, http::header,
	http::header::ContentType, http::header::HeaderValue, middleware::ErrorHandlerResponse,
	middleware::ErrorHandlers, web, App, HttpResponse, HttpServer, Responder,
};
use article::Article;
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

#[get("/")]
async fn root(data: web::Data<GlobalData>, session: Session) -> actix_web::Result<impl Responder> {
	let db = data.get_database();
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Get articles
	let articles = Article::list(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Produce articles HTML
	let mut articles_html = String::new();
	for article in articles {
		let content = article
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;
		if !admin && !content.public {
			continue;
		}

		let post_date = if let Some(post_date) = article.post_date {
			post_date.0.to_rfc3339()
		} else {
			"not posted yet".to_string()
		};

		let mut tags = vec![];

		if admin {
			let pub_tag = if content.public { "Public" } else { "Private" };
			tags.push(pub_tag);
		}

		if content.sponsor {
			tags.push("<i>Sponsors early access</i>&nbsp;❤️");
		}
		if !content.tags.is_empty() {
			tags.extend(content.tags.split(','));
		}

		let tags_html: String = tags
			.into_iter()
			.map(|s| format!(r#"<li class="tag">{s}</li>"#))
			.collect();

		articles_html.push_str(&format!(
			r#"<a href="{article_path}">
				<div class="article-element">
					<img class="article-cover" src="{article_cover_url}"></img>
					<div class="article-element-content">
						<h3>{article_title}</h3>

						<ul class="tags">
							<li><h6 style="color: gray;"><span id="date">{post_date}</span></h6></li>
							{tags_html}
						</ul>

						<p>
							{article_desc}
						</p>
					</div>
				</div>
			</a>"#,
			article_cover_url = content.cover_url,
			article_path = content.get_path(),
			article_title = content.title,
			article_desc = content.desc,
		));
	}

	let html = include_str!("../pages/index.html");
	let html = html.replace("{discord.invite}", &data.discord_invite);
	let html = html.replace("{articles}", &articles_html);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}

#[get("/bio")]
async fn bio() -> impl Responder {
	let html = include_str!("../pages/bio.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/legal")]
async fn legal() -> impl Responder {
	let html = include_str!("../pages/legal.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

#[get("/robots.txt")]
async fn robots() -> impl Responder {
	r#"User-agent: *
Allow: /
Sitemap: https://blog.lenot.re/sitemap.xml"#
}

#[get("/sitemap.xml")]
async fn sitemap(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let mut urls = vec![];

	urls.push(("/".to_owned(), None));
	urls.push(("/bio".to_owned(), None));
	urls.push(("/legal".to_owned(), None));

	let db = data.get_database();
	let articles = Article::list(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	for a in articles {
		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;

		urls.push((content.get_url(), Some(content.edit_date)));
	}

	let urls: String = urls
		.into_iter()
		.map(|(url, date)| match date {
			Some(date) => {
				let date = date.format("%Y-%m-%d");
				format!("\t\t<url><loc>{url}</loc><lastmod>{date}</lastmod></url>")
			}

			None => format!("\t\t<url><loc>{url}</loc></url>"),
		})
		.collect();

	let body = format!(
		r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
	{urls}
</urlset>"#
	);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::xml())
		.body(body))
}

#[get("/rss")]
async fn rss(data: web::Data<GlobalData>) -> actix_web::Result<impl Responder> {
	let db = data.get_database();
	let articles = Article::list(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let mut items_str = String::new();
	for a in articles {
		let Some(ref post_date) = a.post_date else {
			continue;
		};
		let post_date = post_date.0.to_rfc2822();

		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;
		let url = content.get_url();

		items_str.push_str(&format!(
			"<item><guid>{url}</guid><title>{title}</title><link>{url}</link><pubDate>{post_date}</pubDate><description>{desc}</description><author>llenotre</author></item>",
			title = content.title,
			desc = content.desc
		));
	}

	let body = format!(
		r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel><atom:link href="https://blog.lenot.re/rss" rel="self" type="application/rss+xml" /><title>Luc Lenôtre</title><link>https:/blog.lenot.re/</link><description>A blog about writing an operating system from scratch in Rust.</description>{items_str}</channel></rss>"#
	);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::xml())
		.body(body))
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
			.service(bio)
			.service(route::comment::delete)
			.service(route::comment::edit)
			.service(route::comment::post)
			.service(route::file::delete)
			.service(route::file::get)
			.service(route::file::manage)
			.service(route::file::upload)
			.service(legal)
			.service(newsletter::subscribe)
			.service(robots)
			.service(root)
			.service(rss)
			.service(sitemap)
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
