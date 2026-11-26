//! The country a client connected from, for analytics.
//!
//! A [`ClientOrigin`] is read off a request's headers before its body is
//! consumed: the client ip ([`analytics_ext::client_ip`]) and the country a
//! fronting proxy named in `cf-ipcountry`. The proxy's header is authoritative
//! when sent, and is trusted exactly as `client_ip` trusts `cf-connecting-ip`:
//! only an origin reachable solely through the proxy can rely on it, since any
//! client reaching the origin directly can set either header. Two ascii letters
//! are its country; `XX` (unknown), `T1` (Tor) or anything else is no country,
//! never a fall through to the database, which would find a Tor exit's country.
//!
//! Without the header the country is [`GeoIp`]'s lookup of the ip, an offline
//! MaxMind-format `.mmdb`, so analytics buckets by country without collecting or
//! storing a raw ip. Best-effort: with the `geoip` feature off, no database
//! declared, or an unresolvable ip, the country is `None` and callers omit it.
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

/// The header a fronting Cloudflare proxy names the client's country in.
const PROXY_COUNTRY_HEADER: &str = "cf-ipcountry";

/// Where a client connected from: its ip, and the country a fronting proxy
/// named, see the [module docs](self) for the trust model.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ClientOrigin {
	/// The client ip, `None` when the transport exposed none.
	pub ip: Option<IpAddr>,
	/// The proxy's `cf-ipcountry` verdict: `None` without the header,
	/// `Some(None)` when it named no country (`XX`, `T1`, malformed).
	proxy_country: Option<Option<SmolStr>>,
}

impl ClientOrigin {
	/// Reads the client ip and the proxy's country from request headers.
	pub fn from_headers(headers: &HeaderMap) -> Self {
		Self {
			ip: analytics_ext::client_ip(headers),
			proxy_country: headers
				.first_raw(PROXY_COUNTRY_HEADER)
				.map(Self::parse_proxy_country),
		}
	}

	/// An origin known only by its ip, ie an ssh peer, which no proxy fronts.
	pub fn from_ip(ip: Option<IpAddr>) -> Self {
		Self {
			ip,
			proxy_country: None,
		}
	}

	/// Whether resolving the country needs a database: no proxy verdict, and an
	/// ip to look up.
	pub fn needs_lookup(&self) -> bool {
		self.proxy_country.is_none() && self.ip.is_some()
	}

	/// The client's ISO 3166-1 alpha-2 country: the proxy's verdict when it sent
	/// one, else `geoip`'s lookup of the ip. `geoip` is called only when
	/// [`Self::needs_lookup`], so a caller resolves its database lazily.
	pub fn country<'a>(
		&self,
		geoip: impl FnOnce() -> Option<&'a GeoIp>,
	) -> Option<SmolStr> {
		match (&self.proxy_country, self.ip) {
			(Some(verdict), _) => verdict.clone(),
			(None, Some(ip)) => geoip()?.country(ip),
			(None, None) => None,
		}
	}

	/// Two ascii letters uppercased, excluding `XX`; anything else is no country.
	fn parse_proxy_country(value: &str) -> Option<SmolStr> {
		let value = value.trim();
		(value.len() == 2
			&& value.bytes().all(|byte| byte.is_ascii_alphabetic()))
		.then(|| value.to_ascii_uppercase())
		.filter(|code| code != "XX")
		.map(SmolStr::from)
	}
}

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
	pub src: RelPath,
}

impl Default for GeoIpDb {
	fn default() -> Self {
		Self {
			src: RelPath::new(COUNTRY_DB_PATH),
		}
	}
}

impl GeoIpDb {
	/// Observer: load the declared database from the nearest ancestor
	/// [`BlobStore`] and insert the resulting [`GeoIp`] on this entity.
	///
	/// Async so the ancestry has settled by the time the store is resolved: a
	/// markup scene establishes `ChildOf` after the components land.
	#[cfg(feature = "json")]
	pub(super) fn load_on_add(ev: On<Add<GeoIpDb>>, commands: AsyncCommands) {
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
	source: Option<std::sync::Arc<CountrySource>>,
}

/// What a loaded [`GeoIp`] answers from.
enum CountrySource {
	/// A parsed `.mmdb`.
	#[cfg(feature = "geoip")]
	Mmdb(CountryDb),
	/// A fixed ip to country table.
	Table(HashMap<IpAddr, SmolStr>),
}

impl GeoIp {
	/// Loads the country database at `path` from `assets`, the store the app
	/// serves its other content from (the checkout in dev, the app bucket when
	/// deployed).
	///
	/// A declared-but-missing blob is a `warn!` yielding an empty [`GeoIp`] whose
	/// lookups return `None`; a blob that is present but unparseable is an error,
	/// since a corrupt database is a shipping mistake rather than a degraded mode.
	pub async fn load(assets: &BlobStore, path: &RelPath) -> Result<Self> {
		cfg_if! {
			if #[cfg(feature = "geoip")] {
				let Ok(bytes) = assets.get(path).await else {
					warn!("geoip: no country database at {path} in {}, lookups disabled", assets.root_key());
					return Self::default().xok();
				};
				let db = CountryDb::from_bytes(bytes.to_vec())?;
				debug!("geoip: country database loaded from {path}");
				Self::from_source(CountrySource::Mmdb(db)).xok()
			} else {
				let _ = (assets, path);
				Self::default().xok()
			}
		}
	}

	/// A database answering from a fixed ip to country table, ie a stand-in
	/// for the `.mmdb` in tests. Available without the `geoip` feature.
	pub fn from_table(
		table: impl IntoIterator<Item = (IpAddr, impl Into<SmolStr>)>,
	) -> Self {
		table
			.into_iter()
			.map(|(ip, country)| (ip, country.into()))
			.collect::<HashMap<_, _>>()
			.xmap(CountrySource::Table)
			.xmap(Self::from_source)
	}

	fn from_source(source: CountrySource) -> Self {
		Self {
			source: Some(std::sync::Arc::new(source)),
		}
	}

	/// The ISO 3166-1 alpha-2 country code for `ip`, or `None` when the database
	/// is unavailable or the ip is unresolvable.
	pub fn country(&self, ip: IpAddr) -> Option<SmolStr> {
		match self.source.as_deref()? {
			#[cfg(feature = "geoip")]
			CountrySource::Mmdb(db) => db.country(ip),
			CountrySource::Table(table) => table.get(&ip).cloned(),
		}
	}

	/// [`Self::country`] from a string ip, parsing it first. A non-ip string
	/// (or a `host:port` authority) yields `None`.
	pub fn country_str(&self, ip: &str) -> Option<SmolStr> {
		self.country(ip.parse().ok()?)
	}

	/// Whether a database is loaded; `false` yields `None` for every lookup.
	pub fn is_loaded(&self) -> bool { self.source.is_some() }
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

	/// Headers carrying `ip` as the client address and, when given, the proxy's
	/// country.
	fn headers(ip: &str, country: Option<&str>) -> HeaderMap {
		let mut headers = HeaderMap::default();
		headers.set_raw("cf-connecting-ip", ip);
		if let Some(country) = country {
			headers.set_raw(PROXY_COUNTRY_HEADER, country);
		}
		headers
	}

	/// A stand-in database resolving the test ip to `NZ`.
	fn nz_geoip() -> GeoIp {
		GeoIp::from_table([("203.0.113.7".parse::<IpAddr>().unwrap(), "NZ")])
	}

	/// The proxy's header wins over the database, uppercased; without one the
	/// database answers.
	#[beet_core::test]
	fn proxy_country_precedes_the_database() {
		let geoip = nz_geoip();
		ClientOrigin::from_headers(&headers("203.0.113.7", Some(" us ")))
			.country(|| Some(&geoip))
			.xpect_eq(Some("US".into()));
		ClientOrigin::from_headers(&headers("203.0.113.7", None))
			.country(|| Some(&geoip))
			.xpect_eq(Some("NZ".into()));
	}

	/// A proxy that names no country is still authoritative: its unknowns never
	/// fall through to the database, which would find a Tor exit's country.
	#[beet_core::test]
	fn proxy_unknowns_are_no_country() {
		for unknown in ["XX", "T1", "", "USA", "1A"] {
			let origin = ClientOrigin::from_headers(&headers(
				"203.0.113.7",
				Some(unknown),
			));
			origin.needs_lookup().xpect_false();
			origin
				.country(|| panic!("looked up despite the proxy's {unknown:?}"))
				.xpect_none();
		}
	}

	/// No ip and no header means nothing to look up, so the database is never
	/// resolved.
	#[beet_core::test]
	fn no_lookup_without_an_ip() {
		let origin = ClientOrigin::from_headers(&HeaderMap::default());
		origin.needs_lookup().xpect_false();
		origin
			.country(|| panic!("looked up without an ip"))
			.xpect_none();
		let geoip = nz_geoip();
		ClientOrigin::from_ip(Some("203.0.113.7".parse().unwrap()))
			.country(|| Some(&geoip))
			.xpect_eq(Some("NZ".into()));
	}

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
		let assets = BlobStore::new(InMemoryBlobStore::new());
		GeoIp::load(&assets, &RelPath::new(COUNTRY_DB_PATH))
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
		let assets = BlobStore::new(InMemoryBlobStore::new());
		let path = RelPath::new(COUNTRY_DB_PATH);
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
			AbsPath::new_workspace_rel(".").unwrap(),
		));
		let path = RelPath::new(COUNTRY_DB_PATH);
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
