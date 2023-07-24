//! This module implements analytics.

use crate::service::analytics::AnalyticsEntry;
use crate::GlobalData;
use actix_session::Session;
use actix_session::SessionExt;
use actix_web::dev::forward_ready;
use actix_web::dev::Service;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::dev::Transform;
use actix_web::Error;
use chrono::Utc;
use futures_util::future::LocalBoxFuture;
use std::future::ready;
use std::future::Ready;
use std::sync::Arc;

/// Middleware allowing to collect analytics.
pub struct Analytics {
	pub global: Arc<GlobalData>,
}

impl<S, B> Transform<S, ServiceRequest> for Analytics
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Error = Error;
	type Future = Ready<Result<Self::Transform, Self::InitError>>;
	type InitError = ();
	type Response = ServiceResponse<B>;
	type Transform = AnalyticsMiddleware<S>;

	fn new_transform(&self, service: S) -> Self::Future {
		ready(Ok(AnalyticsMiddleware {
			global: self.global.clone(),
			service,
		}))
	}
}

pub struct AnalyticsMiddleware<S> {
	global: Arc<GlobalData>,
	service: S,
}

impl<S, B> Service<ServiceRequest> for AnalyticsMiddleware<S>
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
	S::Future: 'static,
	B: 'static,
{
	type Error = Error;
	type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
	type Response = ServiceResponse<B>;

	forward_ready!(service);

	fn call(&self, req: ServiceRequest) -> Self::Future {
		let (req, payload) = req.into_parts();

		let peer_addr = req
			.connection_info()
			.realip_remote_addr()
			.map(str::to_owned);
		let user_agent = req
			.headers()
			.get("User-Agent")
			.and_then(|h| h.to_str().ok())
			.map(str::to_owned);

		// Get user login, if logged
		let session: Session = req.get_session();
		let logged_user = session.get("user_login").ok().flatten();

		let entry = AnalyticsEntry {
			date: Utc::now(),

			peer_addr,
			user_agent,

			method: req.method().to_string(),
			uri: req.uri().to_string(),

			logged_user,
		};
		let db = self.global.get_database();
		tokio::spawn(async move {
			if let Err(e) = entry.insert(&db).await {
				tracing::error!(error = %e, "cannot log analytics");
			}
		});

		let req = ServiceRequest::from_parts(req, payload);
		Box::pin(self.service.call(req))
	}
}
