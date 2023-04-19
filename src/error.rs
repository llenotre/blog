//! TODO doc

use actix_web::Error;
use actix_web::HttpResponseBuilder;
use actix_web::body::EitherBody;
use actix_web::dev::Service;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::dev::Transform;
use actix_web::dev::forward_ready;
use actix_web::http::header::ContentType;
use futures_util::future::LocalBoxFuture;
use std::future::Ready;
use std::future::ready;

/// Middleware handling errors.
pub struct ErrorHandling;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandling
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
	B: 'static
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlingMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorHandlingMiddleware {
			service
		}))
    }
}

/// TODO doc
pub struct ErrorHandlingMiddleware<S> {
	service: S,
}

impl<S, B> Service<ServiceRequest> for ErrorHandlingMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
	B: 'static
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

	forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
		let fut = self.service.call(req);

		Box::pin(async move {
			let res = fut.await;

			match res {
				Ok(response) => Ok(response.map_body(|_, body| EitherBody::Left { body })),

				Err(e) => {
					let status = e.as_response_error().status_code();

					let html = include_str!("../pages/error.html");
					let html = html.replace("{code}", status.as_str());

					let reason = status.canonical_reason().unwrap_or("Unknown");
					let html = html.replace("{reason}", reason);

					let res = HttpResponseBuilder::new(status)
						.content_type(ContentType::html())
						.body(html);

					Ok(req.into_response(res).map_body(|_, body| EitherBody::Right { body }))
				}
			}
		})
	}
}
