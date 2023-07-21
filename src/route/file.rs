//! This module implements files upload and usage.

use crate::GlobalData;
use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{
	error, get, http::header::ContentType, post, web, web::Redirect, HttpResponse, Responder,
};
use bson::doc;
use bson::oid::ObjectId;
use futures_util::AsyncWriteExt;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use mongodb::options::GridFsFindOptions;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::io::ReaderStream;
use crate::service::user::User;

#[get("/file/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
	let id = ObjectId::parse_str(id.into_inner()).map_err(|_| error::ErrorBadRequest(""))?;
	let db = data.get_database();

	let bucket = db.gridfs_bucket(None);
	// TODO handle case when file doesn't exist
	let stream = bucket
		.open_download_stream(id.into())
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?
		.compat();
	let stream = ReaderStream::new(stream);

	// TODO add mime type? (no available here)
	Ok(HttpResponse::Ok().streaming(stream))
}

#[get("/file")]
pub async fn manage(
	data: web::Data<GlobalData>,
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

	let html = include_str!("../../pages/file_manage.html");

	let bucket = db.gridfs_bucket(None);

	let files = bucket
		.find(
			doc! {},
			Some(
				GridFsFindOptions::builder()
					.sort(doc! { "uploadDate": -1 })
					.build(),
			),
		)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	let files_html = files
		.map(|file| {
			let file = file.unwrap(); // TODO handle error
			let id = file.id.as_object_id().unwrap().to_hex();
			let filename = file.filename.as_deref().unwrap_or("<i>no name</i>");
			let len = file.length;
			let date: chrono::DateTime<chrono::Utc> = file.upload_date.into();

			// TODO if picture, show it as background? (mime type is not available here)

			format!(
				r#"<div class="article" style="background: #2f2f2f;">
					<h2><a href="/file/{id}" target="_blank">{filename}</a></h2>

					<p>Size: {len} bytes</p>
					<p>Uploaded at: {date} (UTC)</p>

					<p><a href="/file/{id}/delete">Delete</a></p>
				</div>"#,
				date = date.format("%d/%m/%Y %H:%M:%S")
			)
		})
		.collect::<String>()
		.await;
	let html = html.replace("{file.list}", &files_html);

	Ok(HttpResponse::Ok()
		.content_type(ContentType::html())
		.body(html))
}

#[post("/file/upload")]
pub async fn upload(
	data: web::Data<GlobalData>,
	mut multipart: Multipart,
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

	let bucket = db.gridfs_bucket(None);

	loop {
		let res = multipart
			.try_next()
			.await
			.map_err(|_| error::ErrorInternalServerError(""));
		let Some(mut field) = res? else {
			break;
		};

		let Some(filename) = field.content_disposition().get_filename() else {
			continue;
		};

		// Upload file to database
		let mut db_stream = bucket.open_upload_stream(filename, None);
		while let Some(chunk) = field.next().await {
			let chunk = chunk?;
			db_stream.write_all(&chunk).await?;
		}
		db_stream.close().await?;
	}

	// Redirect user
	Ok(Redirect::to("/file").see_other())
}

#[get("/file/{id}/delete")]
pub async fn delete(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	let id = id.into_inner();
	let id = ObjectId::parse_str(id).map_err(|_| error::ErrorBadRequest(""))?;

	let db = data.get_database();

	// Check auth
	let admin = User::check_admin(&db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

	// Delete file
	let bucket = db.gridfs_bucket(None);
	bucket
		.delete(id.into())
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;

	// Redirect user
	Ok(Redirect::to("/file").see_other())
}
