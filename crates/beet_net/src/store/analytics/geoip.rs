//! Offline geoip country lookup for analytics.
//!
//! Derives an ISO country code from a client ip using a committed MaxMind-format
//! `.mmdb`, so analytics can bucket by country without collecting or storing a
//! raw ip. Best-effort: with the `geoip` feature off, or no database present, or
//! an unresolvable ip, [`GeoIp::country`] returns `None` and callers omit the
//! country.
//!
//! The database is a static asset at `databases/country.mmdb` under the assets
//! store (the local `assets/` dir, or the deployed assets bucket). Use a
//! redistributable database (db-ip Lite or IP2Location LITE, both CC-licensed)
//! rather than MaxMind GeoLite2, whose license restricts committing the file.
use crate::prelude::*;
use beet_core::prelude::*;
use std::net::IpAddr;

/// The relative path of the country database under the assets directory.
#[cfg(feature = "geoip")]
const COUNTRY_DB_PATH: &str = "databases/country.mmdb";

/// Resource wrapping the loaded country database, or empty when unavailable.
#[derive(Default, Clone, Resource)]
pub struct GeoIp {
	#[cfg(feature = "geoip")]
	db: Option<std::sync::Arc<GeoIpDb>>,
}

impl GeoIp {
	/// Loads the country database from `databases/country.mmdb` in the assets
	/// store, so a deployed site reads the same bucket-backed store it serves
	/// assets from (a local fs path would be empty in a container).
	///
	/// Best-effort: a missing blob, an unreadable blob, or a parse failure logs
	/// and yields an empty [`GeoIp`] whose lookups return `None`, rather than
	/// failing analytics setup.
	pub async fn load(assets: &BlobStore) -> Self {
		cfg_if! {
			if #[cfg(feature = "geoip")] {
				let path = SmolPath::new(COUNTRY_DB_PATH);
				match assets.get(&path).await {
					Ok(bytes) => match GeoIpDb::from_bytes(bytes.to_vec()) {
						Ok(db) => {
							debug!("geoip: country database loaded from {path}");
							Self { db: Some(std::sync::Arc::new(db)) }
						}
						Err(err) => {
							warn!("geoip: failed to parse {path}: {err}");
							Self::default()
						}
					},
					Err(_) => {
						debug!("geoip: no country database at {path}, lookups disabled");
						Self::default()
					}
				}
			} else {
				let _ = assets;
				Self::default()
			}
		}
	}

	/// The ISO 3166-1 alpha-2 country code for `ip`, or `None` when the database
	/// is unavailable or the ip is unresolvable.
	pub fn country(&self, ip: IpAddr) -> Option<SmolStr> {
		cfg_if! {
			if #[cfg(feature = "geoip")] {
				self.db.as_ref()?.country(ip)
			} else {
				let _ = ip;
				None
			}
		}
	}

	/// [`Self::country`] from a string ip, parsing it first. A non-ip string
	/// (or a `host:port` authority) yields `None`.
	pub fn country_str(&self, ip: &str) -> Option<SmolStr> {
		self.country(ip.parse().ok()?)
	}
}

/// The parsed MaxMind-format country database.
#[cfg(feature = "geoip")]
struct GeoIpDb {
	reader: maxminddb::Reader<Vec<u8>>,
}

#[cfg(feature = "geoip")]
impl GeoIpDb {
	/// Parses an owned `.mmdb` byte buffer into a reader.
	fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
		maxminddb::Reader::from_source(bytes)
			.map(|reader| Self { reader })
			.map_err(|err| bevyhow!("invalid mmdb: {err}"))
	}

	/// Looks up the country for `ip`, copying its iso code out of the borrowed
	/// record. A missing record (ip not in the database) yields `None`.
	fn country(&self, ip: IpAddr) -> Option<SmolStr> {
		let country: maxminddb::geoip2::Country = self.reader.lookup(ip).ok()?;
		country.country?.iso_code.map(SmolStr::from)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Without a loaded database, lookups return `None` rather than erroring, so
	/// analytics degrades gracefully when no db is committed.
	#[beet_core::test]
	fn empty_geoip_returns_none() {
		let geoip = GeoIp::default();
		geoip.country_str("8.8.8.8").xpect_none();
		geoip.country_str("not-an-ip").xpect_none();
	}

	/// The committed country database resolves a known ip to its country, proving
	/// the real mmdb loads and the lookup path works end to end.
	#[cfg(feature = "geoip")]
	#[beet_core::test]
	async fn resolves_known_ip_from_committed_db() {
		let assets = BlobStore::new(FsStore::new(
			AbsPathBuf::new_workspace_rel("assets").unwrap(),
		));
		let geoip = GeoIp::load(&assets).await;
		// google public dns is a stable US address in the db-ip country database.
		geoip.country_str("8.8.8.8").xpect_eq(Some("US".into()));
		// an unroutable/private address has no country record.
		geoip.country_str("10.0.0.1").xpect_none();
	}
}
