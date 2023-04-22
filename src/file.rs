//! This module implements files upload and usage.

use actix_session::Session;
use actix_web::{
	HttpResponse,
	Responder,
	get,
	http::header::ContentType,
	post,
	web,
	web::Redirect,
};
use actix_multipart::Multipart;
use bson::doc;
use bson::oid::ObjectId;
use crate::GlobalData;
use crate::user::User;
use futures_util::AsyncWriteExt;
use futures_util::StreamExt;
use serde::Deserialize;

/// Payload for file upload.
#[derive(Deserialize)]
pub struct FileUpload {
	/// The name of the file.
	name: String,
}

#[get("/file/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> impl Responder {
	let id = ObjectId::parse_str(&id.into_inner()).unwrap(); // TODO handle error
	let db = data.get_database();

	let bucket = db.gridfs_bucket(None);
	let _stream = bucket.open_download_stream(id.into()).await.unwrap(); // TODO handle error

	// TODO
	//HttpResponse::Ok().streaming(stream)
	HttpResponse::Ok().finish()
}

#[get("/file")]
pub async fn manage(
	data: web::Data<GlobalData>,
	session: Session,
) -> impl Responder {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session).await.unwrap(); // TODO handle error
	if !admin {
		// TODO
		todo!();
	}

	let html = include_str!("../pages/file_manage.html");

	let bucket = db.gridfs_bucket(None);

	let files = bucket.find(doc!{}, None).await.unwrap(); // TODO handle error
	let files_html = files.map(|file| {
			let file = file.unwrap(); // TODO handle error
			let id = file.id.as_object_id().unwrap().to_hex();

			// TODO if picture, show it as background

			format!(
				r#"<div class="article" style="background: #2f2f2f;">
					<h2><a href="/file/{}" target="_blank">{}</a></h2>

					<p>Size: {} bytes</p>

					<p><a href="/file/{}/delete">Delete</a></p>
				</div>"#,
				id,
				file.filename
					.as_ref()
					.map(|s| s.as_str())
					.unwrap_or("<i>no name</i>"),
				file.length,
				id
			)
		})
		.collect::<String>()
		.await;
	let html = html.replace("{file.list}", &files_html);

	HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html)
}

// TODO if uploaded file has size zero, cancel
#[post("/file/upload")]
pub async fn upload(
	data: web::Data<GlobalData>,
	mut multipart: Multipart,
	session: Session,
) -> impl Responder {
	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session).await.unwrap(); // TODO handle error
	if !admin {
		// TODO
		todo!();
	}

	let mut file_stream = multipart.next().await.unwrap().unwrap(); // TODO handle error
	let file_name = "TODO"; // TODO

	let bucket = db.gridfs_bucket(None);

	// Upload file to database
	let mut db_stream = bucket.open_upload_stream(file_name, None);
	while let Some(chunk) = file_stream.next().await {
		let chunk = chunk.unwrap(); // TODO handle error
		db_stream.write_all(&chunk).await.unwrap(); // TODO handle error
	}
	db_stream.close().await.unwrap(); // TODO handle error

	// Redirect user
	Redirect::to("/file").see_other()
}
