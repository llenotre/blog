//! Insertion and aggregation of analytics.
//!
//! Some data that are collected on users are sensitive and need to be removed past a certain delay
//! to comply with the GDPR.

use crate::util::{now, PgResult};
use anyhow::Result;
use chrono::Duration;
use chrono::NaiveDateTime;
use serde::Serialize;
use std::cell::OnceCell;
use std::fs::File;
use std::net::IpAddr;
use std::sync::Mutex;
use tracing::warn;
use uaparser::{Parser, UserAgentParser};

/// Information about a user's geolocation.
#[derive(Serialize)]
pub struct UserGeolocation {
	city: Option<String>,
	continent: Option<String>,
	country: Option<String>,

	latitude: Option<f64>,
	longitude: Option<f64>,
	accuracy_radius: Option<u16>,
	time_zone: Option<String>,
}

impl TryFrom<IpAddr> for UserGeolocation {
	type Error = anyhow::Error;

	fn try_from(addr: IpAddr) -> Result<Self, Self::Error> {
		static GEOIP_DB: Mutex<OnceCell<maxminddb::Reader<Vec<u8>>>> = Mutex::new(OnceCell::new());
		let geoip_db = GEOIP_DB.lock().unwrap();
		let geoip_db = geoip_db.get_or_init(|| {
			maxminddb::Reader::open_readfile("analytics/geoip.mmdb")
				.expect("could not read geoip database")
		});
		let geolocation: maxminddb::geoip2::City = geoip_db.lookup(addr)?;

		Ok(UserGeolocation {
			city: geolocation
				.city
				.and_then(|c| c.names)
				.as_ref()
				.and_then(|n| n.get("en").or_else(|| n.values().next()))
				.map(|s| (*s).to_owned()),
			continent: geolocation
				.continent
				.and_then(|c| c.code)
				.map(str::to_owned),
			country: geolocation
				.country
				.and_then(|c| c.iso_code)
				.map(str::to_owned),

			latitude: geolocation.location.as_ref().and_then(|c| c.latitude),
			longitude: geolocation.location.as_ref().and_then(|c| c.longitude),
			accuracy_radius: geolocation
				.location
				.as_ref()
				.and_then(|c| c.accuracy_radius),
			time_zone: geolocation
				.location
				.as_ref()
				.and_then(|c| c.time_zone)
				.map(str::to_owned),
		})
	}
}

/// Informations about a user's device.
#[derive(Serialize)]
pub struct UserDevice {
	device_family: String,
	device_brand: Option<String>,
	device_model: Option<String>,

	os_family: String,
	os_major: Option<String>,
	os_minor: Option<String>,
	os_patch: Option<String>,
	os_patch_minor: Option<String>,

	agent_family: String,
	agent_major: Option<String>,
	agent_minor: Option<String>,
}

impl TryFrom<&str> for UserDevice {
	type Error = anyhow::Error;

	fn try_from(user_agent: &str) -> Result<Self, Self::Error> {
		static UA_PARSER: Mutex<OnceCell<UserAgentParser>> = Mutex::new(OnceCell::new());
		let ua_parser = UA_PARSER.lock().unwrap();
		let ua_parser = ua_parser.get_or_init(|| {
			let file = File::open("analytics/uaparser.yaml")
				.expect("could not read user agent parser regexes file");
			UserAgentParser::from_file(file).expect("invalid user agent parser regexes")
		});
		let parsed = ua_parser.parse(user_agent);

		Ok(UserDevice {
			device_family: parsed.device.family.into(),
			device_brand: parsed.device.brand.map(Into::into),
			device_model: parsed.device.model.map(Into::into),

			os_family: parsed.os.family.into(),
			os_major: parsed.os.major.map(Into::into),
			os_minor: parsed.os.minor.map(Into::into),
			os_patch: parsed.os.patch.map(Into::into),
			os_patch_minor: parsed.os.patch_minor.map(Into::into),

			agent_family: parsed.user_agent.family.into(),
			agent_major: parsed.user_agent.major.map(Into::into),
			agent_minor: parsed.user_agent.minor.map(Into::into),
		})
	}
}

/// Each time a page is visited, an instance of this structure is saved.
pub struct AnalyticsEntry {
	/// The date of visit.
	date: NaiveDateTime,

	/// The user's IP address.
	///
	/// If unknown or removed, the value is `None`.
	peer_addr: Option<IpAddr>,
	/// The user agent.
	///
	/// If unknown or removed, the value is `None`
	user_agent: Option<String>,
	/// The referer URL given by the client.
	referer: Option<String>,

	/// Information about the user's geolocation.
	///
	/// If unknown, the value is `None`.
	geolocation: Option<UserGeolocation>,
	/// Information about the user's device.
	///
	/// If unknown, the value is `None`.
	device: Option<UserDevice>,

	/// The request method.
	method: String,
	/// The request URI.
	uri: String,
}

impl AnalyticsEntry {
	pub fn new(
		peer_addr: Option<IpAddr>,
		user_agent: Option<String>,
		referer: Option<String>,
		method: String,
		uri: String,
	) -> Self {
		// Get geolocation from peer address
		let geolocation =
			peer_addr.and_then(|peer_addr| match UserGeolocation::try_from(peer_addr) {
				Ok(l) => Some(l),
				Err(e) => {
					warn!(%peer_addr, error = %e, "could not retrieve user's location");
					None
				}
			});
		// Parse user agent
		let device =
			user_agent
				.as_deref()
				.and_then(|user_agent| match UserDevice::try_from(user_agent) {
					Ok(l) => Some(l),
					Err(e) => {
						warn!(user_agent, error = %e, "could not retrieve information about user's device");
						None
					}
				});

		Self {
			date: now(),

			peer_addr,
			user_agent,
			referer,

			geolocation,
			device,

			method,
			uri,
		}
	}

	/// Inserts the analytics entry in the database.
	pub async fn insert(&self, db: &tokio_postgres::Client) -> Result<()> {
		db.execute(
			"INSERT INTO analytics (date, peer_addr, user_agent, referer, geolocation, device, method, uri)\
				VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
			&[
				&self.date,
				&self.peer_addr,
				&self.user_agent,
				&self.referer,
				&self
					.geolocation
					.as_ref()
					.map(serde_json::to_value)
					.transpose()?,
				&self.device.as_ref().map(serde_json::to_value).transpose()?,
				&self.method,
				&self.uri,
			],
		)
		.await?;
		Ok(())
	}

	/// Anonymizes entries.
	pub async fn aggregate(db: &tokio_postgres::Client) -> PgResult<()> {
		// The end of the date range in which entries are going to be anonymized
		let end = now() - Duration::hours(24);
		db.execute(
			"UPDATE analytics SET peer_addr = NULL, user_agent = NULL WHERE date <= $1 AND (peer_addr IS NOT NULL OR user_agent IS NOT NULL)",
			&[&end],
		)
		.await?;
		Ok(())
	}
}
