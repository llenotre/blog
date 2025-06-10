use axum::{
	extract::Request,
	http::{StatusCode, header},
	middleware::Next,
	response::{IntoResponse, Response},
};

pub async fn redirect(req: Request, next: Next) -> Result<Response, StatusCode> {
	let host = req
		.headers()
		.get(header::HOST)
		.and_then(|h| h.to_str().ok());
	// If the domain is correct, no redirection is needed
	if host == Some("blog.lenot.re") {
		return Ok(next.run(req).await);
	}
	// Else, redirect
	let url = req.uri().path_and_query().map_or_else(
		|| "https://blog.lenot.re".to_string(),
		|uri| format!("https://blog.lenot.re{uri}"),
	);
	Ok((StatusCode::PERMANENT_REDIRECT, [(header::LOCATION, url)]).into_response())
}
