//! Offline geoip country lookup for analytics.
//!
//! Derives an ISO country code from a client ip using a MaxMind-format `.mmdb`,
//! so analytics can bucket by country without collecting or storing a raw ip.
//! Best-effort: with the `geoip` feature off, or no database declared, or an
//! unresolvable ip, [`GeoIp::country`] returns `None` and callers omit the
//! country.
//!
//! The database is *content*: it ships with the document as a static asset, so
//! `<GeoIpDb/>` resolves it out of the nearest ancestor [`BlobStore`] (the
//! checkout in dev, the app bucket when deployed), exactly the way `<AssetsDir/>`
//! resolves the files it serves. The file itself is gitignored and hydrated by
//! `just site-shared pull`. Use a redistributable database (db-ip Lite or
//! IP2Location LITE, both CC-licensed) rather than MaxMind GeoLite2, whose
//! license restricts redistributing the file.
use crate::prelude::*;
use beet_core::prelude::*;
use std::net::IpAddr;

/// The default path of the country database, relative to the repo store root.
const COUNTRY_DB_PATH: &str = "assets/databases/country.mmdb";

/// Declares the country database an analytics surface looks its visitors up in:
/// `<GeoIpDb/>`, spread on the router beside its [`AnalyticsConfig`].
///
/// The database is content rather than a provisioned resource, so it is named by
/// a path under the nearest ancestor [`BlobStore`] and never by a `StoreRef`
/// relation. The load inserts a [`GeoIp`] on this same entity, so consumers
/// (the request middleware, the web beacon) resolve it by ancestry.
///
/// No `GeoIpDb` means no country, by authorship: analytics still records, with
/// an empty country field.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct GeoIpDb {
	/// The database path, relative to the nearest ancestor [`BlobStore`] root.
	pub src: SmolPath,
}

impl Default for GeoIpDb {
	fn default() -> Self {
		Self {
			src: SmolPath::new(COUNTRY_DB_PATH),
		}
	}
}

impl GeoIpDb {
	/// Observer: load the declared database from the nearest ancestor
	/// [`BlobStore`] and insert the resulting [`GeoIp`] on this entity.
	///
	/// Async so the ancestry has settled by the time the store is resolved: a
	/// markup scene establishes `ChildOf` after the components land.
	pub(super) fn load_on_add(ev: On<Add, GeoIpDb>, commands: AsyncCommands) {
		let entity = ev.entity;
		commands.run(async move |world| {
			let entity = world.entity(entity);
			let src = entity.get::<Self, _>(|db| db.src.clone()).await?;
			let store = entity
				.with_state::<AncestorQuery<&BlobStore>, _>(
					|entity, stores| stores.get(entity).cloned().ok(),
				)
				.await?;
			let geoip = match store {
				Some(store) => GeoIp::load(&store, &src).await?,
				None => {
					warn!(
						"geoip: <GeoIpDb src=\"{src}\"/> has no ancestor BlobStore to read from, lookups disabled"
					);
					GeoIp::default()
				}
			};
			entity.insert(geoip).await?;
			Ok(())
		});
	}
}

/// Component wrapping the loaded country database, or empty when unavailable.
/// Inserted by [`GeoIpDb`] on its own entity, so consumers resolve it by
/// ancestry.
#[derive(Default, Clone, Component)]
pub struct GeoIp {
	#[cfg(feature = "geoip")]
	db: Option<std::sync::Arc<CountryDb>>,
}

impl GeoIp {
	/// Loads the country database at `path` from `assets`, the store the app
	/// serves its other content from (the checkout in dev, the app bucket when
	/// deployed).
	///
	/// A declared-but-missing blob is a `warn!` yielding an empty [`GeoIp`] whose
	/// lookups return `None`; a blob that is present but unparseable is an error,
	/// since a corrupt database is a shipping mistake rather than a degraded mode.
	pub async fn load(assets: &BlobStore, path: &SmolPath) -> Result<Self> {
		cfg_if! {
			if #[cfg(feature = "geoip")] {
				let Ok(bytes) = assets.get(path).await else {
					warn!("geoip: no country database at {path} in {}, lookups disabled", assets.root_key());
					return Self::default().xok();
				};
				let db = CountryDb::from_bytes(bytes.to_vec())?;
				debug!("geoip: country database loaded from {path}");
				Self { db: Some(std::sync::Arc::new(db)) }.xok()
			} else {
				let _ = (assets, path);
				Self::default().xok()
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

	/// Whether a database is loaded; `false` yields `None` for every lookup.
	pub fn is_loaded(&self) -> bool {
		cfg_if! {
			if #[cfg(feature = "geoip")] { self.db.is_some() } else { false }
		}
	}
}

/// The reader itself is an opaque 8 MB buffer, so this reports only whether one
/// is loaded.
impl std::fmt::Debug for GeoIp {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.debug_struct("GeoIp")
			.field("loaded", &self.is_loaded())
			.finish()
	}
}

/// The parsed MaxMind-format country database.
#[cfg(feature = "geoip")]
struct CountryDb {
	reader: maxminddb::Reader<Vec<u8>>,
}

#[cfg(feature = "geoip")]
impl CountryDb {
	/// Parses an owned `.mmdb` byte buffer into a reader.
	fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
		maxminddb::Reader::from_source(bytes)
			.map(|reader| Self { reader })
			.map_err(|err| bevyhow!("invalid mmdb: {err}"))
	}

	/// Looks up the country for `ip`, copying its iso code out of the borrowed
	/// record. A missing record (ip not in the database) yields `None`.
	fn country(&self, ip: IpAddr) -> Option<SmolStr> {
		let country = self
			.reader
			.lookup(ip)
			.ok()?
			.decode::<maxminddb::geoip2::Country>()
			.ok()??;
		country.country.iso_code.map(SmolStr::from)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Without a loaded database, lookups return `None` rather than erroring, so
	/// analytics degrades gracefully when none is declared.
	#[beet_core::test]
	fn empty_geoip_returns_none() {
		let geoip = GeoIp::default();
		geoip.country_str("8.8.8.8").xpect_none();
		geoip.country_str("not-an-ip").xpect_none();
	}

	/// A declared database that is not in the store degrades to empty lookups
	/// rather than failing the load.
	#[beet_core::test]
	async fn missing_db_yields_empty_lookups() {
		let assets = BlobStore::new(InMemoryStore::new());
		GeoIp::load(&assets, &SmolPath::new(COUNTRY_DB_PATH))
			.await
			.unwrap()
			.country_str("8.8.8.8")
			.xpect_none();
	}

	/// A blob that is present but not an mmdb is a shipping mistake, so it errors
	/// rather than silently disabling every country.
	#[cfg(feature = "geoip")]
	#[beet_core::test]
	async fn corrupt_db_is_loud() {
		let assets = BlobStore::new(InMemoryStore::new());
		let path = SmolPath::new(COUNTRY_DB_PATH);
		assets.insert(&path, "not an mmdb").await.unwrap();
		GeoIp::load(&assets, &path)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("invalid mmdb");
	}

	/// The real country database resolves a known ip to its country, proving the
	/// mmdb loads and the lookup path works end to end. The store roots at the
	/// workspace, the same shape as a repo store containing `assets/`. The file is
	/// gitignored, so an unhydrated checkout skips rather than fails.
	#[cfg(feature = "geoip")]
	#[beet_core::test]
	async fn resolves_known_ip_from_the_real_db() {
		let assets = BlobStore::new(FsStore::new(
			AbsPathBuf::new_workspace_rel(".").unwrap(),
		));
		let path = SmolPath::new(COUNTRY_DB_PATH);
		if !assets.exists(&path).await.unwrap() {
			warn!(
				"skipping: no {path} in this checkout, run `just site-shared pull` to hydrate it"
			);
			return;
		}
		let geoip = GeoIp::load(&assets, &path).await.unwrap();
		// google public dns is a stable US address in the db-ip country database.
		geoip.country_str("8.8.8.8").xpect_eq(Some("US".into()));
		// an unroutable/private address has no country record.
		geoip.country_str("10.0.0.1").xpect_none();
	}
}
