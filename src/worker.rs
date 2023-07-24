//! The worker is an object which is ticked at a regular interval in order to perform database
//! maintainance tasks.

use crate::service::analytics::AnalyticsEntry;
use crate::GlobalData;
use std::sync::Arc;

pub struct Worker {
	/// Global data
	data: Arc<GlobalData>,
}

impl Worker {
	/// Creates a new instance.
	pub fn new(data: Arc<GlobalData>) -> Self {
		Self {
			data,
		}
	}

	/// Ticks the worker.
	pub async fn tick(&self) {
		let _ = AnalyticsEntry::aggregate(&self.data.get_database()).await;
	}
}
