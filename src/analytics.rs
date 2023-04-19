//! This module implements analytics.

use actix_web::Error;
use actix_web::dev::Service;
use actix_web::dev::ServiceRequest;
use actix_web::dev::ServiceResponse;
use actix_web::dev::Transform;
use actix_web::dev::forward_ready;
use chrono::DateTime;
use chrono::Utc;
use crate::GlobalData;
use crate::util;
use futures_util::future::LocalBoxFuture;
use serde::Deserialize;
use serde::Serialize;
use std::future::Ready;
use std::future::ready;
use std::sync::Arc;

/// Each time a page is visited, an instance of this structure is saved.
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEntry {
	/// The date of the visit.
	#[serde(with = "util::serde_date_time")]
	date: DateTime<Utc>,

	/// The user's address.
	address: String,
	/// The user agent.
	user_agent: Option<String>,

	/// The request method.
	method: String,
	/// The request URI.
	uri: String,

	/// If a user is logged, the name of this user.
	logged_user: Option<String>,
}

impl AnalyticsEntry {
	/// Inserts the analytics entry in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");
		collection.insert_one(self, None)
			.await
			.map(|_| ())
	}
}

/// Middleware allowing to collect analytics.
pub struct Analytics {
	pub global: Arc<GlobalData>,
}

impl<S, B> Transform<S, ServiceRequest> for Analytics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
	B: 'static
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AnalyticsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AnalyticsMiddleware {
			global: self.global.clone(),
			service
		}))
    }
}

/// TODO doc
pub struct AnalyticsMiddleware<S> {
	global: Arc<GlobalData>,
	service: S,
}

impl<S, B> Service<ServiceRequest> for AnalyticsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
	B: 'static
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

	forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
		let request = req.request();
		if let Some(addr) = request.peer_addr() {
			let entry = AnalyticsEntry {
				date: chrono::offset::Utc::now(),

				address: format!("{}", addr),
				user_agent: request.headers()
					.get("User-Agent")
					.map(|h| h.to_str().ok())
					.flatten()
					.map(|h| h.to_owned()),

				method: format!("{}", request.method()),
				uri: format!("{}", request.uri()),

				logged_user: None, // TODO get from current session
			};

			let db = (*self.global).mongo.database("blog");
			tokio::spawn(async move {
				if let Err(e) = entry.insert(&db).await {
					eprintln!("Cannot log analytics: {}", e);
				}
			});
		}

		let fut = self.service.call(req);
		Box::pin(async move {
			fut.await
		})
	}
}
