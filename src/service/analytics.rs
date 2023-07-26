//! TODO doc

use crate::util;
use bson::doc;
use bson::oid::ObjectId;
use chrono::Utc;
use chrono::{DateTime, Duration};
use futures_util::StreamExt;
use serde::Deserialize;
use serde::Serialize;
use std::cell::OnceCell;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Mutex;
use uaparser::{Parser, UserAgentParser};

/// Geoip database.
static GEOIP_DB: Mutex<OnceCell<maxminddb::Reader<&'static [u8]>>> = Mutex::new(OnceCell::new());
/// The user agent parser.
static UA_PARSER: Mutex<OnceCell<UserAgentParser>> = Mutex::new(OnceCell::new());

/// Informations about a user's geolocation.
#[derive(Deserialize, Serialize)]
pub struct UserGeolocation {
	city: Option<String>,
	continent: Option<String>,
	country: Option<String>,

	latitude: Option<f64>,
	longitude: Option<f64>,
	accuracy_radius: Option<u16>,
	time_zone: Option<String>,
}

/// Informations about a user's device.
#[derive(Deserialize, Serialize)]
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

/// Informations about the user. This is an enumeration because client data has to be aggregated on a regular basis for GDPR reasons.
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum UserInfo {
	/// Sensitive data, not aggregated yet.
	Sensitive {
		/// The user's IP address. If unknown or removed, the value is `None`.
		peer_addr: Option<String>,
		/// The user agent. If unknown or removed, the value is `None`
		user_agent: Option<String>,
	},
	/// Aggregated data.
	Aggregated {
		/// Informations about the user's geolocation.
		geolocation: Option<UserGeolocation>,
		/// Informations about the user's device.
		device: Option<UserDevice>,
	},
}

/// Each time a page is visited, an instance of this structure is saved.
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEntry {
	/// The entry's ID.
	#[serde(rename = "_id")]
	pub id: ObjectId,

	/// The date of the visit.
	#[serde(with = "util::serde_date_time")]
	pub date: DateTime<Utc>,

	/// User's info.
	pub user_info: UserInfo,

	/// The request method.
	pub method: String,
	/// The request URI.
	pub uri: String,
}

impl AnalyticsEntry {
	/// Inserts the analytics entry in the database.
	///
	/// `db` is the database.
	pub async fn insert(&self, db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");

		let peer_addr = match &self.user_info {
			UserInfo::Sensitive { peer_addr, .. } => peer_addr.as_deref(),
			_ => None,
		};
		let entry = collection
			.find_one(
				doc! {
					"peer_addr": peer_addr,
					"uri": &self.uri,
				},
				None,
			)
			.await?;
		// Do not count the same client twice
		if entry.is_none() {
			collection.insert_one(self, None).await?;
		}

		Ok(())
	}

	/// Aggregates entries.
	///
	/// `db` is the database.
	pub async fn aggregate(db: &mongodb::Database) -> Result<(), mongodb::error::Error> {
		let collection = db.collection::<Self>("analytics");

		let oldest = Utc::now() - Duration::hours(24);
		// Get the list of entries to aggregate
		let mut entries_iter = collection
			.find(
				doc! {
					"date": { "$lt": oldest },
					"user_info.kind": "Sensitive"
				},
				None,
			)
			.await?;
		while let Some(mut e) = entries_iter.next().await.transpose()? {
			let UserInfo::Sensitive {
				peer_addr,
				user_agent,
			} = e.user_info else {
				continue;
			};

			// Get geolocation from peer address
			let geolocation = peer_addr
				.and_then(|addr| IpAddr::from_str(&addr).ok())
				.and_then(|addr| {
					let geoip_db = GEOIP_DB.lock().unwrap();
					let geoip_db = geoip_db.get_or_init(|| {
						let db = include_bytes!("../../analytics/geoip.mmdb");
						maxminddb::Reader::from_source(db.as_slice())
							.expect("invalid geoip database")
					});
					let geolocation: maxminddb::geoip2::City = geoip_db.lookup(addr).ok()?;

					Some(UserGeolocation {
						// TODO check correctness
						city: geolocation
							.city
							.and_then(|c| c.names)
							.as_ref()
							.and_then(|n| n.get("en"))
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
				});

			// Parse user agent
			let device = user_agent.map(|user_agent| {
				let ua_parser = UA_PARSER.lock().unwrap();
				let ua_parser = ua_parser.get_or_init(|| {
					let yaml = include_bytes!("../../analytics/uaparser.yaml");
					UserAgentParser::from_bytes(yaml).expect("invalid user agent parser regexes")
				});
				let parsed = ua_parser.parse(&user_agent);

				UserDevice {
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
				}
			});

			let user_info = UserInfo::Aggregated {
				geolocation,
				device,
			};

			// Update entry
			collection
				.update_one(
					doc! {
						"_id": e.id,
					},
					doc! {
						"$set": { "user_info": bson::to_bson(&user_info)? }
					},
					None,
				)
				.await?;
		}

		Ok(())
	}
}
