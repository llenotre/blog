mod article;
mod util;

use actix_files::Files;
use actix_web::{
	HttpResponse,
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
use std::sync::Mutex;

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
	mongo: mongodb::Client,
}

/// Query specifying the current page.
#[derive(Deserialize)]
pub struct PageQuery {
	/// The current page number.
	page: u32,
}

#[get("/")]
async fn root(data: web::Data<Mutex<GlobalData>>, page: web::Query<PageQuery>) -> impl Responder {
	let page = page.into_inner().page;

	// Article colors
	static colors: [&str; 5] = [
		"#ea2027", // red
		"#ee5a24", // orange
		"#009432", // green
		"#0652dd", // blue
		"#833471" // purple
	];

	// Get articles
	let articles = {
		let db = data.lock().unwrap().mongo.database("blog");
		Article::list(&db, page, 10)
			.await
			.unwrap() // TODO handle error (http 500)
	};

	// Produce articles HTML
	let articles_html: String = articles.into_iter()
		.enumerate()
		.map(|(i, article)| {
			let color = colors[i % colors.len()];

			format!(
				r#"<div class="article" style="background-color: {};">
					<h2><a href="/article/{}">{}</a><h2>

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
	let html = html.replace("{articles}", &articles_html);

	HttpResponse::Ok().body(html)
}

#[get("/article/{id}")]
async fn get_article(id: web::Path<String>) -> impl Responder {
	let _article_id = id.into_inner();

	// TODO read page code
	// TODO get article (if not found, 404)
	// TODO generate HTML from markdown
	// TODO replace tag in page
	HttpResponse::Ok().body("TODO")
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

    let data = web::Data::new(Mutex::new(GlobalData {
		mongo: client,
	}));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new("[%t] %a: %r - Response: %s (in %D ms)"))
            .app_data(data.clone())
            .service(Files::new("/assets", "./assets"))
            .service(root)
            .service(get_article)
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
