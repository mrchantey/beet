//! The store-selection grammar behind `--store` / `BEET_STORE`.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// Which blob store an entry loads through: a value type with a grammar rather
/// than a string predicate, so selection is parsed once and every consumer asks
/// the type instead of re-matching a prefix.
///
/// Parse, display and predicates only, so it needs no store backend and no
/// features; `BlobStore::from_uri` in beet_net constructs the backend it names.
/// The grammar round-trips through [`Display`]:
///
/// 1. `fs` / `fs:<path>`: a filesystem store, rooted at the resolution context
///    dir (the resolved entry dir for an entry store) or at `<path>` when given,
///    relative paths resolved against the context dir.
/// 2. `memory`: a temporary in-memory store.
/// 3. `s3://<bucket>[?endpoint=<url>][&region=<region>]`: an S3-compatible
///    bucket. With an `endpoint` (eg a Cloudflare R2 account endpoint) the region
///    defaults to `auto`; without one it falls back to `AWS_REGION`, then
///    `us-west-2`.
/// 4. `local-storage` / `indexed-db`: browser storage (wasm).
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let uri = StoreUri::parse("s3://my-site?region=us-east-1").unwrap();
/// uri.is_self_rooted().xpect_true();
/// uri.to_string().xpect_eq("s3://my-site?region=us-east-1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StoreUri {
	/// A filesystem store. `path` overrides the resolution context dir, else the
	/// store roots at the context dir itself.
	Fs {
		/// The store root, absolute or relative to the resolution context dir.
		path: Option<SmolStr>,
	},
	/// A temporary in-memory store, only meaningful with an explicit entry since
	/// it has no seeded entry to discover.
	Memory,
	/// An S3-compatible bucket, the store a deployed task serves from.
	S3 {
		/// The bucket name.
		bucket: SmolStr,
		/// An S3-compatible service endpoint, eg
		/// `https://<account>.r2.cloudflarestorage.com`. Selects region `auto`
		/// by default, so one binary serves identically on AWS S3 and R2.
		endpoint: Option<SmolStr>,
		/// The bucket region, resolved at construction when absent.
		region: Option<SmolStr>,
	},
	/// Browser `localStorage` (wasm).
	LocalStorage,
	/// Browser IndexedDB (wasm).
	IndexedDb,
}

impl Default for StoreUri {
	fn default() -> Self { Self::Fs { path: None } }
}

impl StoreUri {
	/// Parse a store uri, erroring with the supported list on an unknown kind.
	pub fn parse(value: &str) -> Result<Self> {
		let value = value.trim();
		if let Some(rest) = value.strip_prefix("s3://") {
			return Self::parse_s3(rest);
		}
		if let Some(path) = value.strip_prefix("fs:") {
			let path = path.trim();
			return Self::Fs {
				path: (!path.is_empty()).then(|| path.into()),
			}
			.xok();
		}
		match value {
			"fs" => Self::Fs { path: None },
			"memory" => Self::Memory,
			"local-storage" => Self::LocalStorage,
			"indexed-db" => Self::IndexedDb,
			other => bevybail!(
				"unknown store `{other}`, supported kinds: fs, fs:<path>, \
				memory, s3://<bucket>[?endpoint=..][&region=..], local-storage \
				(wasm), indexed-db (wasm)"
			),
		}
		.xok()
	}

	/// Parse the `<bucket>[?endpoint=<url>][&region=<region>]` tail of an `s3://`
	/// uri.
	fn parse_s3(rest: &str) -> Result<Self> {
		let (bucket, query) = rest.split_once('?').unwrap_or((rest, ""));
		if bucket.is_empty() {
			bevybail!("store `s3://` is missing a bucket name");
		}
		let mut endpoint = None;
		let mut region = None;
		for pair in query.split('&').filter(|pair| !pair.is_empty()) {
			match pair.split_once('=') {
				Some(("endpoint", value)) => endpoint = Some(value.into()),
				Some(("region", value)) => region = Some(value.into()),
				_ => bevybail!(
					"unknown s3 store query param `{pair}`, supported: \
					endpoint, region"
				),
			}
		}
		Self::S3 {
			bucket: bucket.into(),
			endpoint,
			region,
		}
		.xok()
	}

	/// Whether this store roots itself (a bucket or browser storage), needing no
	/// local directory or filesystem walk. For these an entry name addresses the
	/// document *within* the store and there is no live-reload watch dir.
	pub fn is_self_rooted(&self) -> bool {
		matches!(self, Self::S3 { .. } | Self::LocalStorage | Self::IndexedDb)
	}
}

impl FromStr for StoreUri {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> { Self::parse(value) }
}

impl fmt::Display for StoreUri {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Fs { path: None } => write!(f, "fs"),
			Self::Fs { path: Some(path) } => write!(f, "fs:{path}"),
			Self::Memory => write!(f, "memory"),
			Self::LocalStorage => write!(f, "local-storage"),
			Self::IndexedDb => write!(f, "indexed-db"),
			Self::S3 {
				bucket,
				endpoint,
				region,
			} => {
				write!(f, "s3://{bucket}")?;
				// the query separator is `?` for the first param, `&` after, so
				// the rendered uri parses back into this same value.
				let mut sep = '?';
				if let Some(endpoint) = endpoint {
					write!(f, "{sep}endpoint={endpoint}")?;
					sep = '&';
				}
				if let Some(region) = region {
					write!(f, "{sep}region={region}")?;
				}
				Ok(())
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Every kind parses and renders back to the exact text it parsed from.
	#[crate::test]
	fn round_trips() {
		for uri in [
			"fs",
			"fs:../site",
			"fs:/abs/site",
			"memory",
			"local-storage",
			"indexed-db",
			"s3://my-bucket",
			"s3://my-bucket?region=us-east-1",
			"s3://my-bucket?endpoint=https://acc.r2.cloudflarestorage.com",
			"s3://my-bucket?endpoint=https://acc.r2.cloudflarestorage.com&region=auto",
		] {
			StoreUri::parse(uri).unwrap().to_string().xpect_eq(uri);
		}
	}

	#[crate::test]
	fn parses_fields() {
		StoreUri::parse("fs")
			.unwrap()
			.xpect_eq(StoreUri::Fs { path: None });
		StoreUri::parse("fs:site").unwrap().xpect_eq(StoreUri::Fs {
			path: Some("site".into()),
		});
		StoreUri::parse("s3://b?endpoint=http://e&region=r")
			.unwrap()
			.xpect_eq(StoreUri::S3 {
				bucket: "b".into(),
				endpoint: Some("http://e".into()),
				region: Some("r".into()),
			});
	}

	/// Only a bucket or browser storage is self-rooted; a filesystem store needs
	/// its resolution context dir.
	#[crate::test]
	fn self_rooted_kinds() {
		StoreUri::parse("s3://b")
			.unwrap()
			.is_self_rooted()
			.xpect_true();
		StoreUri::parse("local-storage")
			.unwrap()
			.is_self_rooted()
			.xpect_true();
		StoreUri::parse("indexed-db")
			.unwrap()
			.is_self_rooted()
			.xpect_true();
		StoreUri::parse("fs")
			.unwrap()
			.is_self_rooted()
			.xpect_false();
		StoreUri::parse("memory")
			.unwrap()
			.is_self_rooted()
			.xpect_false();
	}

	#[crate::test]
	fn rejects_malformed() {
		StoreUri::parse("nope")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown store");
		StoreUri::parse("s3://")
			.unwrap_err()
			.to_string()
			.xpect_contains("missing a bucket name");
		StoreUri::parse("s3://b?nope=1")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown s3 store query param");
	}
}
