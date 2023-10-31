//! This module implements files upload and usage.

use crate::GlobalData;
use actix_web::{
	error, get, web, HttpResponse, Responder,
};
use tracing::error;

#[get("/file/{uuid}")]
pub async fn get(
	data: web::Data<GlobalData>,
	uuid: web::Path<String>,
) -> actix_web::Result<impl Responder> {
	let uuid = uuid.into_inner();
	let query = format!("COPY (SELECT data FROM file WHERE uuid = '{uuid}') TO STDOUT BINARY");
	let stream = data.db.read().await.copy_out(&query).await.map_err(|e| {
		error!(error = %e, "postgres: open download stream");
		error::ErrorInternalServerError("")
	})?;
	// TODO mime type
	Ok(HttpResponse::Ok().streaming(stream))
}
