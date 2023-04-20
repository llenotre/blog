//! This module handles articles.

use actix_web::{
	HttpResponse,
	Responder,
	get,
	http::header::ContentType,
	post,
	web,
};
use actix_session::Session;
use bson::Bson;
use bson::oid::ObjectId;
use chrono::DateTime;
use chrono::Utc;
use crate::GlobalData;
use crate::comment::Comment;
use crate::user;
use crate::util;
use futures_util::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use serde::Deserialize;
use serde::Serialize;

/// Structure representing an article.
#[derive(Serialize, Deserialize)]
pub struct Article {
	/// The article's id.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The article's title.
	pub title: String,
	/// The article's description.
	pub desc: String,

	// TODO keep history
	/// The article's content.
	pub content: String,

	/// Timestamp since epoch at which the article has been posted.
	#[serde(with = "util::serde_date_time")]
	pub post_date: DateTime<Utc>,

	/// Tells whether the article is public.
	pub public: bool,
	/// Tells whether comments are locked on the article.
	pub comments_locked: bool,
}

impl Article {
	/// Returns the total number of articles.
	pub async fn get_total_count(db: &mongodb::Database) -> Result<u32, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.count_documents(None, None)
			.await
			.map(|n| n as _)
	}

	/// Returns the list of articles for the given page.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `page` is the page number.
	/// - `per_page` is the number of articles per page.
	/// - `public` tells whether to the function must return only public articles.
	pub async fn list(
		db: &mongodb::Database,
		page: u32,
		per_page: u32,
		public: bool
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		let find_options = FindOptions::builder()
			.skip(Some((page * per_page) as _))
			.limit(Some(per_page as _))
			.sort(Some(doc!{
				"post_date": -1
			}))
			.build();

		let filter = if public {
			Some(doc!{
				"public": true,
			})
		} else {
			None
		};

		collection.find(
			filter,
			Some(find_options)
		)
			.await?
			.try_collect()
			.await
	}

	/// Returns the article with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the article.
	pub async fn get(
		db: &mongodb::Database,
		id: ObjectId
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.find_one(Some(doc!{"_id": id}), None).await
	}

	/// Inserts the current article in the database.
	///
	/// `db` is the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(
		&self,
		db: &mongodb::Database
	) -> Result<Bson, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.insert_one(self, None)
			.await
			.map(|r| r.inserted_id)
	}
}

#[get("/article/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> impl Responder {
	let id_str = id.into_inner();
	session.insert("last_article", id_str.clone()).unwrap(); // TODO handle error

	let id = ObjectId::parse_str(&id_str).unwrap(); // TODO handle error (http 404)

	// Get article
	let (article, comments) = {
		let db = data.get_database();

		let article = Article::get(&db, id)
			.await
			.unwrap(); // TODO handle error (http 500)
		let comments = Comment::list_for_article(&db, id)
			.await
			.unwrap(); // TODO handle error (http 500)

		(article, comments)
	};
	let user_login = session.get::<String>("user_login").unwrap(); // TODO handle error

	match article {
		Some(article) => {
			let markdown = markdown::to_html(&article.content);

			let html = include_str!("../pages/article.html");
			let html = html.replace("{article.id}", &id_str);
			let html = html.replace("{article.title}", &article.title);
			let html = html.replace("{article.desc}", &article.desc);
			let html = html.replace("{article.content}", &markdown);

			let comment_editor_html = match user_login {
				Some(user_login) => format!(
					r#"<p>You are currently logged as <b>{}</b>. <a href="/logout">Logout</a></p>

					<h6>Markdown is supported</h6>

					<textarea id="comment" placeholder="What are your thoughts?"></textarea>
					<button id="comment-submit" href="\#">Post comment</button>"#,
					user_login
				),

				None => format!(
					r#"<p><a href="{}">Login</a> with Github to leave a comment.</p>"#,
					user::get_auth_url(&data)
				),
			};
			let html = html.replace("{comment.editor}", &comment_editor_html);

			let comments_count = comments.len();
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
			let html = html.replace("{comments.count}", &format!("{}", comments_count));

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
pub async fn post(
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

			let db = data.get_database();
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
async fn editor(
	data: web::Data<GlobalData>,
	query: web::Query<EditorQuery>,
	session: Session,
) -> impl Responder {
	let _query = query.into_inner();

	// Check auth
	// TODO

	// TODO get article from ID if specified

	let html = include_str!("../pages/editor.html");
	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}
