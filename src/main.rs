mod article;
mod comment;
mod util;

use actix_files::Files;
use actix_web::{
	HttpResponse,
	http::header::ContentType,
	post,
	web,
    App,
    HttpServer,
    Responder,
    get,
    middleware,
};
use article::Article;
use bson::oid::ObjectId;
use comment::Comment;
use comment::CommentContent;
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

#[get("/article/{id}")]
async fn get_article(data: web::Data<GlobalData>, id: web::Path<String>) -> impl Responder {
	let id = id.into_inner();
	let id = ObjectId::parse_str(id).unwrap(); // TODO handle error (http 404)

	// Get article
	let (article, comments) = {
		let db = data.mongo.database("blog");

		let article = Article::get(&db, id)
			.await
			.unwrap(); // TODO handle error (http 500)
		let comments = Comment::list_for_article(&db, id)
			.await
			.unwrap(); // TODO handle error (http 500)

		(article, comments)
	};

	match article {
		Some(article) => {
			let markdown = markdown::to_html(&article.content);

			let html = include_str!("../pages/article.html");
			let html = html.replace("{article.title}", &article.title);
			let html = html.replace("{article.desc}", &article.desc);
			let html = html.replace("{article.content}", &markdown);

			let html = html.replace("{comments.count}", &format!("{}", comments.len()));

			let comments_html: String = comments.into_iter()
				.map(|com| {
					let content = "TODO"; // TODO
					let markdown = markdown::to_html(content);

					format!(r#"<div class="comment">
							<div class="comment-header">
								{} (posted at {})
							</div>

							{}
						</div>"#,
						com.author,
						com.post_date.format("%d/%m/%Y %H:%M:%S"),
						markdown
					)
				})
				.collect();
			let html = html.replace("{comments}", &comments_html);

			HttpResponse::Ok()
				.content_type(ContentType::html())
				.body(html)
		}

		None => {
			// TODO 404
			todo!();
		}
	}
}

/// TODO doc
#[derive(Deserialize)]
pub struct PostCommentPayload {
	/// The ID of the article.
	article_id: String,
	/// The ID of the comment this comment responds to. If `None`, this comment is not a response.
	response_to: Option<ObjectId>,

	/// The content of the comment in markdown.
	content: String,
}

#[post("/article")]
async fn post_comment(
	data: web::Data<GlobalData>,
	payload: web::Json<PostCommentPayload>
) -> impl Responder {
	let payload = payload.into_inner();
	let article_id = ObjectId::parse_str(payload.article_id).unwrap(); // TODO handle error (http 404)

	let id = ObjectId::new();
	let date = chrono::offset::Utc::now();

	let comment = Comment {
		id,

		article: article_id,
		response_to: payload.response_to,

		author: "TODO".to_string(), // TODO

		post_date: date,

		removed: false,
	};
	let comment_content = CommentContent {
		comment_id: id,

		edit_date: date,
	};

	{
		let db = data.mongo.database("blog");

		comment_content.insert(&db)
			.await
			.unwrap(); // TODO handle error (http 500)

		comment.insert(&db)
			.await
			.unwrap(); // TODO handle error (http 500)
	}

	HttpResponse::Ok().finish()
}

/// Article edition coming from the editor.
#[derive(Deserialize)]
pub struct ArticleEdit {
	/// The ID of the article. If `None`, a new article is being created.
	id: Option<String>,

	/// The title of the article.
	title: String,
	/// The description of the article.
	desc: String,

	/// The content of the article in markdown.
	content: String,

	/// Tells whether to publish the article.
	public: String,
}

#[post("/article")]
async fn post_article(
	data: web::Data<GlobalData>,
	info: web::Form<ArticleEdit>
) -> impl Responder {
	let info = info.into_inner();

	let id = match info.id {
		// Update article
		Some(id) => {
			// TODO update article

			id
		}

		// Create article
		None => {
			let a = Article {
				id: ObjectId::new(),

				title: info.title,
				desc: info.desc,

				content: info.content,

				post_date: chrono::offset::Utc::now(),

				public: info.public == "on",
				comments_locked: false,
			};

			let db = data.mongo.database("blog");
			let id = a.insert(&db).await.unwrap(); // TODO handle error

			id.as_object_id().unwrap().to_string()
		}
	};

	web::Redirect::to(format!("/article/{}", id))
}

/// Editor page query.
#[derive(Deserialize)]
pub struct EditorQuery {
	/// The ID of the article to edit. If `None`, a new article is being created.
	id: Option<u32>,
}

#[get("/editor")]
async fn editor(data: web::Data<GlobalData>, query: web::Query<EditorQuery>) -> impl Responder {
	let _query = query.into_inner();

	// TODO check auth
	// TODO get article from ID if specified

	let html = include_str!("../pages/editor.html");
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
            .service(root)
            .service(get_article)
			.service(post_article)
			.service(post_comment)
            .service(editor)
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}
