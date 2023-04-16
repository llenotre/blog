mod article;
mod comment;
mod util;

use actix_files::Files;
use actix_web::{
	HttpResponse,
	http::header::ContentType,
	web,
    App,
    HttpServer,
    Responder,
    get,
    middleware,
};
use article::Article;
use mongodb::Client;
use mongodb::options::ClientOptions;
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
}

/// Structure shared accross the server.
pub struct GlobalData {
	/// The connection to the MongoDB database.
	pub mongo: mongodb::Client,
}

/// Query specifying the current page.
#[derive(Deserialize)]
pub struct PageQuery {
	/// The current page number.
	page: Option<u32>,
}

#[get("/")]
async fn root(data: web::Data<GlobalData>, page: web::Query<PageQuery>) -> impl Responder {
	let page = page.into_inner()
		.page
		.unwrap_or(0);

	// Article colors
	static COLORS: [&str; 5] = [
		"#ea2027", // red
		"#ee5a24", // orange
		"#009432", // green
		"#0652dd", // blue
		"#833471" // purple
	];

	// Get articles
	let (total_articles, articles) = {
		let db = data.mongo.database("blog");

		// TODO handle errors (http 500)
		let total_articles = Article::get_total_count(&db)
			.await
			.unwrap();
		let articles = Article::list(&db, page, ARTICLES_PER_PAGE, true)
			.await
			.unwrap();

		(total_articles, articles)
	};
	let pages_count = util::ceil_div(total_articles, ARTICLES_PER_PAGE);

	if page != 0 && page >= pages_count {
		// TODO http 404
		todo!();
	}

	// Produce articles HTML
	let articles_html: String = articles.into_iter()
		.enumerate()
		.map(|(i, article)| {
			let color = if article.public {
				COLORS[i % COLORS.len()]
			} else {
				"gray"
			};

			format!(
				r#"<div class="article" style="background-color: {};">
					<h2><a href="/article/{}">{}</a></h2>

					<p>
						{}
					</p>
				</div>"#,
				color,
				article.id,
				article.title,
				article.desc
			)
		})
		.collect();

	let html = include_str!("../pages/index.html");
	let html = html.replace("{page.curr}", &format!("{}", page + 1));
	let html = html.replace("{page.total}", &format!("{}", pages_count));
	let html = html.replace("{articles.count}", &format!("{}", total_articles));
	let html = html.replace("{articles}", &articles_html);

	let prev_button_html = if page > 0 {
		format!("<a href=\"?page={}\" class=\"page-button\">Previous Page</a>", page - 1)
	} else {
		String::new()
	};
	let html = html.replace("{button.prev}", &prev_button_html);

	let next_button_html = if page + 1 < pages_count {
		format!("<a href=\"?page={}\" class=\"page-button\" style=\"margin-left: auto;\">Next Page</a>", page + 1)
	} else {
		String::new()
	};
	let html = html.replace("{button.next}", &next_button_html);

	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
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
	});

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new("[%t] %a: %r - Response: %s (in %D ms)"))
            .app_data(data.clone())
            .service(Files::new("/assets", "./assets"))
			.service(article::post)
			.service(comment::post)
            .service(article::editor)
            .service(article::get)
            .service(root)
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
