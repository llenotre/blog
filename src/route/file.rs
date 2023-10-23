//! This module implements files upload and usage.

use crate::service::user::User;
use crate::util::now;
use crate::GlobalData;
use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{
	error, get, http::header::ContentType, post, web, web::Redirect, HttpResponse, Responder,
};
use chrono::NaiveDateTime;
use futures_util::TryStreamExt;
use futures_util::{SinkExt, StreamExt};
use std::iter;
use std::pin::pin;
use tracing::error;
use uuid::Uuid;

#[get("/file/{uuid}")]
pub async fn get(
	data: web::Data<GlobalData>,
	uuid: web::Path<String>,
) -> actix_web::Result<impl Responder> {
	let uuid = uuid.into_inner();
	let query = format!("COPY (SELECT data FROM file WHERE uuid = '{uuid}') TO STDOUT BINARY");
	let stream = data.db.copy_out(&query).await.map_err(|e| {
		error!(error = %e, "postgres: open download stream");
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
			"SELECT uuid,name,upload_date,length(data) as size FROM file ORDER BY upload_date DESC",
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
			let uuid: Uuid = file.get("uuid");
			let name: String = file.get("name");
			let upload_date: NaiveDateTime = file.get("upload_date");
			let size: i32 = file.get("size");

			// TODO if picture, show it as background? (mime type is not available here)

			format!(
				r#"<div class="article" style="background: #2f2f2f;">
					<h2><a href="/file/{uuid}" target="_blank">{name}</a></h2>

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

	let now = now();

	loop {
		let res = multipart
			.try_next()
			.await
			.map_err(|_| error::ErrorInternalServerError(""));
		let Some(field) = res? else {
			break;
		};
		let (Some(filename), Some(mime_type)) = (
			field.content_disposition().get_filename(),
			field.content_type(),
		) else {
			continue;
		};
		let mime_type = mime_type.to_string();

		// Create file in database
		let row = data.db.query_one("INSERT INTO file (uuid, name, mime_type, upload_date, data) VALUES (gen_random_uuid(), $1, $2, $3, '') RETURNING uuid", &[&filename, &mime_type, &now])
			.await
			.map_err(|e| {
				error!(error = %e, "postgres: insert file");
				error::ErrorInternalServerError("")
			})?;
		let uuid: Uuid = row.get("uuid");

		// Send file to database
		let mut in_stream = field.map(|chunk| {
			Ok(chunk.unwrap()) // TODO handle error
		});
		let query = format!("COPY file (data) FROM STDIN BINARY WHERE uuid = '{uuid}'");
		let out_stream = data.db.copy_in(&query).await.map_err(|e| {
			error!(error = %e, "postgres: open upload stream");
			error::ErrorInternalServerError("")
		})?;
		pin!(out_stream)
			.send_all(&mut in_stream)
			.await
			.map_err(|e| {
				error!(error = %e, "postgres: upload stream");
				error::ErrorInternalServerError("")
			})?;
	}

	// Redirect user
	Ok(Redirect::to("/file").see_other())
}
