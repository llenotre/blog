//! This module handles articles.

use crate::comment::comment_to_html;
use crate::comment::get_comment_editor;
use crate::comment::group_comments;
use crate::comment::Comment;
use crate::markdown;
use crate::user;
use crate::user::User;
use crate::util;
use crate::GlobalData;
use actix_session::Session;
use actix_web::{
	error, get, http::header::ContentType, post, web, web::Redirect, HttpResponse, Responder,
};
use bson::oid::ObjectId;
use bson::Bson;
use chrono::DateTime;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use serde::Deserialize;
use serde::Serialize;

// TODO keep previous versions of articles to be able to restore

/// Structure representing an article.
#[derive(Serialize, Debug, Deserialize)]
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
	/// Tells whether the article is reserved for sponsors.
	pub sponsor: bool,

	/// The comma-separated list of tags on the article.
	pub tags: String,

	/// Tells whether comments are locked on the article.
	pub comments_locked: bool,
}

impl Article {
	/// Returns the total number of articles.
	pub async fn get_total_count(db: &mongodb::Database) -> Result<u32, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.count_documents(None, None).await.map(|n| n as _)
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
		public: bool,
	) -> Result<Vec<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		let find_options = FindOptions::builder()
			.skip(Some((page * per_page) as _))
			.limit(Some(per_page as _))
			.sort(Some(doc! {
				"post_date": -1
			}))
			.build();

		let filter = if public {
			Some(doc! {
				"public": true,
			})
		} else {
			None
		};

		collection
			.find(filter, Some(find_options))
			.await?
			.try_collect()
			.await
	}

	/// Returns the article with the given ID.
	///
	/// Arguments:
	/// - `db` is the database.
	/// - `id` is the ID of the article.
	pub async fn from_id(
		db: &mongodb::Database,
		id: &ObjectId,
	) -> Result<Option<Self>, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection.find_one(Some(doc! {"_id": id}), None).await
	}

	/// Inserts the current article in the database.
	///
	/// `db` is the database.
	///
	/// The function returns the ID of the inserted article.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<Bson, mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection
			.insert_one(self, None)
			.await
			.map(|r| r.inserted_id)
	}

	/// Updates the articles with the given ID.
	pub async fn update(
		db: &mongodb::Database,
		id: ObjectId,
		update: bson::Document,
	) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("article");
		collection
			.update_one(doc! { "_id": id }, doc! { "$set": update }, None)
			.await
			.map(|_| ())
	}
}

#[get("/article/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let id_str = id.into_inner();
	session.insert("last_article", id_str.clone())?;

	let id = ObjectId::parse_str(&id_str).map_err(|_| error::ErrorBadRequest(""))?;

	let db = data.get_database();

	// Get article
	let article = Article::from_id(&db, &id)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	match article {
		Some(article) => {
			// If article is not public, the user must be admin to see it
			let admin = User::check_admin(&db, &session)
				.await
				.map_err(|_| error::ErrorInternalServerError(""))?;
			if !article.public && !admin {
				return Err(error::ErrorNotFound(""));
			}
			let html = include_str!("../pages/article.html");
			let html = html.replace("{article.tags}", &article.tags);
			let html = html.replace("{article.id}", &id_str);
			let html = html.replace("{article.title}", &article.title);
			let html = html.replace("{article.desc}", &article.desc);
			let html = html.replace(
				"{article.date}",
				&format!(
					"{}",
					article.post_date.format("%d/%m/%Y %H:%M:%S") // TODO use user's timezone
				),
			);
			let markdown = markdown::to_html(&article.content, false);
			let html = html.replace("{article.content}", &markdown);

			let user_id = session
				.get::<String>("user_id")?
				.map(|id| ObjectId::parse_str(id).map_err(|_| error::ErrorBadRequest("")))
				.transpose()?;
			let user_login = session.get::<String>("user_login")?;

			// Get article reactions
			// TODO
			let html = html.replace("{reactions}", "TODO");

			// Get article comments
			let comments = Comment::list_for_article(&db, id, !admin)
				.await
				.map_err(|_| error::ErrorInternalServerError(""))?;
			let comments_count = comments.len();
			let html = html.replace("{comments.count}", &format!("{}", comments_count));

			let comments = group_comments(comments);
			let mut comments_html = String::new();
			for (com, replies) in comments {
				comments_html.push_str(
					&comment_to_html(
						&db,
						&com,
						Some(&replies),
						user_id.as_ref(),
						&article.id,
						admin,
					)
					.await?,
				);
			}

			let html = html.replace("{comments}", &comments_html);

			let comment_editor_html = match user_login {
				Some(user_login) => format!(
					r#"<h3 id="reply-to" hidden></h3>
					{}"#,
					get_comment_editor(&article.id.to_hex(), "post", None, None)
				),

				None => format!(
					r#"<center><a class="login-button" href="{}"><i class="fa-brands fa-github"></i>&nbsp;&nbsp;&nbsp;Sign in with Github to comment</a></center>"#,
					user::get_auth_url(&data.client_id)
				),
			};
			let html = html.replace("{comment.editor}", &comment_editor_html);

			Ok(HttpResponse::Ok()
				.content_type(ContentType::html())
				.body(html))
		}

		None => Err(error::ErrorNotFound("")),
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
	public: Option<String>,
	/// Tells whether the article is reserved for sponsors.
	sponsor: Option<String>,

	/// The comma-separated list of tags.
	tags: String,
}

#[post("/article")]
pub async fn post(
	data: web::Data<GlobalData>,
	info: web::Form<ArticleEdit>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

	let info = info.into_inner();
	let id = match info.id {
		// Update article
		Some(id_str) => {
			let id = ObjectId::parse_str(&id_str).map_err(|_| error::ErrorBadRequest(""))?;

			Article::update(
				&db,
				id,
				doc! {
					"title": info.title,
					"desc": info.desc,

					"content": info.content,

					"public": info.public.map(|p| p == "on").unwrap_or(false),
					"sponsor": info.sponsor.map(|p| p == "on").unwrap_or(false),

					"tags": info.tags,
				},
			)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?;

			id_str
		}

		// Create article
		None => {
			let a = Article {
				id: ObjectId::new(),

				title: info.title,
				desc: info.desc,

				content: info.content,

				post_date: chrono::offset::Utc::now(),

				public: info.public.map(|p| p == "on").unwrap_or(false),
				sponsor: info.sponsor.map(|p| p == "on").unwrap_or(false),

				tags: info.tags,

				comments_locked: false,
			};

			let db = data.get_database();
			let id = a
				.insert(&db)
				.await
				.map_err(|_| error::ErrorInternalServerError(""))?;

			id.as_object_id().unwrap().to_string()
		}
	};

	// Redirect user
	Ok(Redirect::to(format!("/article/{}", id)).see_other())
}

/// Editor page query.
#[derive(Deserialize)]
pub struct EditorQuery {
	/// The ID of the article to edit. If `None`, a new article is being created.
	id: Option<String>,
}

#[get("/editor")]
async fn editor(
	data: web::Data<GlobalData>,
	query: web::Query<EditorQuery>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorNotFound(""));
	}

	// Get article
	let article_id = query
		.into_inner()
		.id
		.map(ObjectId::parse_str)
		.transpose()
		.map_err(|_| error::ErrorBadRequest(""))?;
	let article = match article_id {
		Some(article_id) => Article::from_id(&db, &article_id)
			.await
			.map_err(|_| error::ErrorInternalServerError(""))?,
		None => None,
	};

	let article_id_html = article
		.as_ref()
		.map(|a| {
			format!(
				"<input name=\"id\" type=\"hidden\" value=\"{}\"></input>",
				a.id.to_hex()
			)
		})
		.unwrap_or_default();
	let article_title = article.as_ref().map(|a| a.title.as_str()).unwrap_or("");
	let article_desc = article.as_ref().map(|a| a.desc.as_str()).unwrap_or("");
	let article_content = article.as_ref().map(|a| a.content.as_str()).unwrap_or("");
	let article_public = article.as_ref().map(|a| a.public).unwrap_or(false);
	let article_sponsor = article.as_ref().map(|a| a.sponsor).unwrap_or(false);
	let article_tags = article.as_ref().map(|a| a.tags.as_str()).unwrap_or("");

	let html = include_str!("../pages/editor.html");
	let html = html.replace("{article.id}", &article_id_html);
	let html = html.replace("{article.title}", article_title);
	let html = html.replace("{article.desc}", article_desc);
	let html = html.replace("{article.content}", article_content);
	let html = html.replace(
		"{article.published}",
		if article_public { "checked" } else { "" },
	);
	let html = html.replace(
		"{article.sponsor}",
		if article_sponsor { "checked" } else { "" },
	);
	let html = html.replace("{article.tags}", article_tags);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}
