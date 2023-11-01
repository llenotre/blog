//! This module implements files upload and usage.

use crate::GlobalData;
use actix_web::{error, get, web, HttpResponse, Responder};
use futures_util::StreamExt;
use tracing::error;

#[get("/file/{name}")]
pub async fn get(
	data: web::Data<GlobalData>,
	name: web::Path<String>,
) -> actix_web::Result<impl Responder> {
	let name = name.into_inner();
	let stream = data
		.s3_bucket
		.get_object_stream(format!("/{name}"))
		.await
		.map_err(|error| {
			error!(%error, "s3 download");
			error::ErrorInternalServerError("")
		})?
		.bytes
		.map(Result::<_, actix_web::error::Error>::Ok);
	Ok(HttpResponse::Ok().streaming(stream))
}
