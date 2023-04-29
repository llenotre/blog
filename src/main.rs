#![feature(async_closure)]

mod analytics;
mod article;
mod comment;
mod file;
mod markdown;
mod user;
mod util;

use crate::user::User;
use actix_files::Files;
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

	// Article colors
	static COLORS: [&str; 2] = ["#006266", "#2f2f2f"];

	let db = data.get_database();
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Get articles
	let total_articles = Article::get_total_count(&db)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let articles = Article::list(&db, page, ARTICLES_PER_PAGE, !admin)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	let pages_count = util::ceil_div(total_articles, ARTICLES_PER_PAGE);
	if page != 0 && page >= pages_count {
		// TODO http 404
		todo!();
	}

	// Produce articles HTML
	let articles_html: String = articles
		.into_iter()
		.enumerate()
		.map(|(i, article)| {
			let color = if article.public {
				COLORS[i % COLORS.len()]
			} else {
				"gray"
			};

			let public_html = match (admin, article.public) {
				(false, _) => "",
				(true, false) => "<h6>PRIVATE</h6>",
				(true, true) => "<h6>PUBLIC</h6>",
			};

			format!(
				r#"<div class="article" style="background-color: {};">
					<h2><a href="/article/{}">{}</a></h2>

					<p>
						{}
					</p>

					<a class="read-button" href="/article/{}">Read <i class="fa-solid fa-arrow-right"></i></a>

					{}
				</div>"#,
				color, article.id, article.title, article.desc, article.id, public_html
			)
		})
		.collect();

	let html = include_str!("../pages/index.html");
	let html = html.replace("{page.curr}", &format!("{}", page + 1));
	let html = html.replace("{page.total}", &format!("{}", pages_count));
	let html = html.replace("{articles.count}", &format!("{}", total_articles));
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

#[get("/legal")]
async fn legal() -> impl Responder {
	let html = include_str!("../pages/legal.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
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

	HttpServer::new(move || {
		App::new()
			.wrap(middleware::Logger::new(
				"[%t] %a: %r - Response: %s (in %D ms)",
			))
			.wrap(analytics::Analytics {
				global: data.clone().into_inner(),
			})
			.wrap(SessionMiddleware::new(
				CookieSessionStore::default(),
				Key::from(config.session_secret_key.as_bytes()), // TODO parse hex
			))
			.wrap(ErrorHandlers::new().default_handler(error_handler))
			.app_data(data.clone())
			.service(Files::new("/assets", "./assets"))
			.service(article::post)
			.service(comment::delete)
			.service(comment::post)
			.service(comment::preview)
			.service(article::editor)
			.service(article::get)
			.service(legal)
			.service(root)
			.service(user::auth)
			.service(user::logout)
			.service(user::oauth)
			.service(file::get)
			.service(file::manage)
			.service(file::upload)
	})
	.bind(format!("0.0.0.0:{}", config.port))?
	.run()
	.await
}
