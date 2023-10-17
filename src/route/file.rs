//! This module implements files upload and usage.

use std::iter;
use crate::service::user::User;
use crate::{GlobalData};
use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{
	error, get, http::header::ContentType, post, web, web::Redirect, HttpResponse, Responder,
};
use futures_util::{StreamExt};
use futures_util::TryStreamExt;
use tracing::error;
use crate::util::Oid;

#[get("/file/{id}")]
pub async fn get(
	data: web::Data<GlobalData>,
	id: web::Path<String>,
) -> actix_web::Result<impl Responder> {
	let id = id.into_inner();
	let query = format!("SELECT data FROM file WHERE id = '{id}'");
	let stream = data
		.db
		.copy_out(&query)
		.await
		.map_err(|e| {
			error!(error = %e, "postgres: file copy out");
			error::ErrorInternalServerError("")
		})?;
	// TODO mime type
	Ok(HttpResponse::Ok().streaming(stream))
}

#[get("/file")]
pub async fn manage(
	data: web::Data<GlobalData>,
	session: Session,
) -> actix_web::Result<impl Responder> {
	// Check auth
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

	let files = data
		.db
		.query_raw(
			"SELECT id,name,upload_date,length(data) as size FROM file ORDER BY upload_date DESC",
			iter::empty::<u32>(),
		)
		.await
		.map_err(|e| {
			error!(error = %e, "postgres: files list");
			error::ErrorInternalServerError("")
		})?;
	let files_html = files
		.map(|file| {
			let file = file.unwrap(); // TODO handle error
			let id: Oid = file.get("id");
			let name: String = file.get("name");
			let upload_date: chrono::DateTime<chrono::Utc> = file.get("upload_date");
			let size: u32 = file.get("size");

			// TODO if picture, show it as background? (mime type is not available here)

			format!(
				r#"<div class="article" style="background: #2f2f2f;">
					<h2><a href="/file/{id}" target="_blank">{name}</a></h2>

					<p>Size: {size} bytes</p>
					<p>Uploaded at: {upload_date} (UTC)</p>
				</div>"#,
				upload_date = upload_date.format("%d/%m/%Y %H:%M:%S")
			)
		})
		.collect::<String>()
		.await;

	let html = include_str!("../../pages/file_manage.html");
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
	// Check auth
	let admin = User::check_admin(&data.db, &session)
		.await
		.map_err(|_| error::ErrorInternalServerError(""))?;
	if !admin {
		return Err(error::ErrorForbidden(""));
	}

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
