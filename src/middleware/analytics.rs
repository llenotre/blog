//! This module implements analytics.

use crate::service::analytics::AnalyticsEntry;
use crate::GlobalData;
use actix_web::dev::forward_ready;
use actix_web::dev::Service;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::dev::Transform;
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use std::future::ready;
use std::future::Ready;
use std::net::IpAddr;
use std::str::FromStr;
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
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Transform = AnalyticsMiddleware<S>;
	type InitError = ();
	type Future = Ready<Result<Self::Transform, Self::InitError>>;

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
	type Response = ServiceResponse<B>;
	type Error = Error;
	type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

	forward_ready!(service);

	fn call(&self, req: ServiceRequest) -> Self::Future {
		let (req, payload) = req.into_parts();

		let peer_addr = req
			.connection_info()
			.realip_remote_addr()
			.map(IpAddr::from_str)
			.transpose()
			.ok()
			.flatten();
		let user_agent = req
			.headers()
			.get("User-Agent")
			.and_then(|h| h.to_str().ok())
			.map(str::to_owned);
		let method = req.method().to_string();
		let uri = req.uri().to_string();

		if !uri.starts_with("/assets") {
			let entry = AnalyticsEntry::new(peer_addr, user_agent, method, uri);
			let global = self.global.clone();
			tokio::spawn(async move {
				if let Err(e) = entry.insert(&*global.db.read().await).await {
					tracing::error!(error = %e, "cannot log analytics");
				}
			});
		}

		let req = ServiceRequest::from_parts(req, payload);
		Box::pin(self.service.call(req))
	}
}
