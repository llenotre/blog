use actix_files::{
	Files,
	NamedFile,
};
use actix_web::{
    get,
    middleware,
    App,
    HttpServer,
    Responder
};
use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::process::exit;

#[derive(Deserialize)]
struct Config {
	/// The HTTP server's port.
	port: u16,
}

#[get("/")]
async fn root() -> impl Responder {
	NamedFile::open_async("./pages/index.html").await
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
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
