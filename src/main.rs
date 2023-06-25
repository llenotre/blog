mod analytics;
mod article;
mod comment;
mod file;
mod markdown;
mod newsletter;
mod user;
mod util;

use crate::newsletter::EmailWorker;
use crate::user::User;
use actix_files::Files;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_session::storage::CookieSessionStore;
use actix_session::Session;
use actix_session::SessionMiddleware;
use actix_web::{
	body::BoxBody, body::EitherBody, cookie::Key, dev::ServiceResponse, error, get, http::header,
	http::header::ContentType, http::header::HeaderValue, middleware,
	middleware::ErrorHandlerResponse, middleware::ErrorHandlers, web, App, HttpResponse,
	HttpServer, Responder,
};
use article::Article;
use mongodb::options::ClientOptions;
use mongodb::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::process::exit;

/// The number of articles per page.
const ARTICLES_PER_PAGE: u32 = 10;

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
}

/// Structure shared accross the server.
pub struct GlobalData {
	/// The connection to the MongoDB database.
	pub mongo: mongodb::Client,

	/// The client ID of the Github application.
	pub client_id: String,
	/// The client secret of the Github application.
	pub client_secret: String,
}

impl GlobalData {
	/// Returns a reference to the database.
	pub fn get_database(&self) -> mongodb::Database {
		self.mongo.database("blog")
	}
}

/// Query specifying the current page.
#[derive(Deserialize)]
pub struct PageQuery {
	/// The current page number.
	page: Option<u32>,
}

#[get("/")]
async fn root(
	data: web::Data<GlobalData>,
	page: web::Query<PageQuery>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let page = page.into_inner().page.unwrap_or(0);

	let db = data.get_database();
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Get articles
	let total_articles = Article::get_total_count(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let articles = Article::list(&db, page, ARTICLES_PER_PAGE)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let pages_count = util::ceil_div(total_articles, ARTICLES_PER_PAGE);
	if page != 0 && page >= pages_count {
		return Err(error::ErrorNotFound(""));
	}

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
		let post_date = article.post_date.format("%d/%m/%Y"); // TODO use user's timezone

		let mut tags = vec![];

		if admin {
			let pub_tag = if content.public { "Public" } else { "Private" };
			tags.push(pub_tag);
		}

		if content.sponsor {
			tags.push("<i>Reserved for Sponsors</i>&nbsp;❤️");
		}
		if !content.tags.is_empty() {
			tags.extend(content.tags.split(','));
		}

		let tags_html: String = tags
			.into_iter()
			.map(|s| format!(r#"<li class="tag">{s}</li>"#))
			.collect();

		articles_html.push_str(&format!(
			r#"<div class="article-element">
				<img class="article-cover" src="{article_cover_url}"></img>
				<div class="article-element-content">
					<h3><a href="{article_path}">{article_title}</a></h3>

					<ul class="tags">
						<li><h6 style="color: gray;">{post_date}</h6></li>
						{tags_html}
					</ul>

					<p>
						{article_desc}
					</p>

					<center>
						<a class="read-button" href="{article_path}">Read more</a>
					</center>
				</div>
			</div>"#,
			article_cover_url = content.cover_url,
			article_path = content.get_path(),
			article_title = content.title,
			article_desc = content.desc,
		));
	}

	let html = include_str!("../pages/index.html");
	let html = html.replace("{articles}", &articles_html);

	let prev_button_html = if page > 0 {
		format!(
			"<a href=\"?page={}\" class=\"button page-button\">Previous Page</a>",
			page - 1
		)
	} else {
		String::new()
	};
	let html = html.replace("{button.prev}", &prev_button_html);

	let next_button_html = if page + 1 < pages_count {
		format!("<a href=\"?page={}\" class=\"button page-button\" style=\"margin-left: auto;\">Next Page</a>", page + 1)
	} else {
		String::new()
	};
	let html = html.replace("{button.next}", &next_button_html);

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
	let articles = Article::list(&db, 0, 100)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	for a in articles {
		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;

		urls.push((content.get_url(), Some(content.edit_date)));
	}

	let urls: String =
		urls.into_iter()
			.map(|(url, date)| match date {
                Some(date) => {
                    let date = date.format("%Y-%m-%d");
                    format!("\t\t<url><loc>{url}</loc><lastmod>{date}</lastmod></url>")
                },

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
	let articles = Article::list(&db, 0, 100)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let mut items_str = String::new();
	for a in articles {
		let date = a.post_date.to_rfc2822();
		let content = a
			.get_content(&db)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;
        let url = content.get_url();

		items_str.push_str(&format!(
			"<item><title>{title}</title><link>{url}</link><pubDate>{date}</pubDate><description>{desc}</description></item>",
			title = content.title,
			desc = content.desc
		));
	}

	let body = format!(
		r#"<rss version="2.0"><channel><title>Luc Lenôtre</title><link>https:/blog.lenot.re/</link><description>A blog about writing an operating system from scratch in Rust.</description>{items_str}</channel></rss>"#
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
	env::set_var("RUST_LOG", "actix_web=info");
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

	// TODO handle errors
	// Open database connection
	let client_options = ClientOptions::parse(&config.mongo_url).await.unwrap();
	let client = Client::with_options(client_options).unwrap();

	let data = web::Data::new(GlobalData {
		mongo: client,

		client_id: config.client_id,
		client_secret: config.client_secret,
	});

    // Run the email worker
    let data_clone = data.clone();
    tokio::spawn(async {
        let email_worker = EmailWorker::new(data_clone);
        email_worker.run().await;
    });

	let governor_conf = GovernorConfigBuilder::default()
		.per_second(1)
		.burst_size(50)
		.finish()
		.unwrap();

	HttpServer::new(move || {
		App::new()
			.wrap(middleware::Logger::new(
				"[%t] %a: %r - Response: %s (in %D ms)",
			))
			//.wrap(Governor::new(&governor_conf))
			.wrap(SessionMiddleware::new(
				CookieSessionStore::default(),
				Key::from(config.session_secret_key.as_bytes()), // TODO parse hex
			))
			.wrap(analytics::Analytics {
				global: data.clone().into_inner(),
			})
			.wrap(ErrorHandlers::new().default_handler(error_handler))
			.app_data(data.clone())
			.app_data(web::PayloadConfig::new(1024 * 1024))
			.service(Files::new("/assets", "./assets"))
			.service(article::editor)
			.service(article::get)
			.service(article::post)
			.service(bio)
			.service(comment::delete)
			.service(comment::edit)
			.service(comment::post)
			.service(file::delete)
			.service(file::get)
			.service(file::manage)
			.service(file::upload)
			.service(legal)
			.service(newsletter::subscribe)
			.service(robots)
			.service(root)
			.service(rss)
			.service(sitemap)
			.service(user::auth)
			.service(user::avatar)
			.service(user::logout)
			.service(user::oauth)
	})
	.bind(format!("0.0.0.0:{}", config.port))?
	.run()
	.await
}
