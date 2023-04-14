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
}

/// Query specifying the current page.
#[derive(Deserialize)]
pub struct PageQuery {
	/// The current page number.
	page: u32,
}

#[get("/")]
async fn root(page: web::Query<PageQuery>) -> impl Responder {
	let curr_page = page.into_inner().page;

	// Article colors
	let colors = [
		"#ea2027", // red
		"#ee5a24", // orange
		"#009432", // green
		"#0652dd", // blue
		"#833471" // purple
	];

	// TODO replace by real articles
	let articles_html: String = (0..10)
		.map(|i| {
			let color = colors[i % colors.len()];

			format!(r#"<div class="article" style="background-color: {};">
				<h2><a href="/article/TODO">Lorem ipsum</a><h2>

				<p>
					Lorem ipsum dolor sit amet, consectetur adipiscing elit</br>
				</p>
			</div>"#, color)
		})
		.collect();

	let html = include_str!("../pages/index.html");
	let html = html.replace("{articles}", &articles_html);

	HttpResponse::Ok().body(html)
}

#[get("/article/{id}")]
async fn article(id: web::Path<String>) -> impl Responder {
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

	// TODO read config from file
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

    //let data = web::Data::new(Mutex::new(GlobalData::new(config)));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new("[%t] %a: %r - Response: %s (in %D ms)"))
            //.app_data(data.clone())
            .service(Files::new("/assets", "./assets"))
            .service(root)
            .service(article)
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
