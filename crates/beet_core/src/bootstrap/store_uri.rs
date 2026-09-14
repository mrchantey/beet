//! The store-selection grammar behind `--repo` / `BEET_REPO` and every store
//! declaration.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// Which store a process reads: a value type with a grammar rather than a
/// string predicate, so selection is parsed once and every consumer asks the
/// type instead of re-matching a prefix.
///
/// Parse, display and predicates only, so it needs no store backend and no
/// features; `StoreProvider::from_uri` in beet_net constructs the backend it
/// names. Every kind is TOTAL: a uri carries everything its backend needs (a
/// bucket, a table, a browser database) plus the key prefix it roots at, so
/// the one uri is the whole answer and nothing downstream patches it. The
/// grammar round-trips through [`Display`]:
///
/// 1. `fs` / `fs:<path>`: a filesystem store, rooted at the context dir (the
///    cwd, or the dir [`rooted_at`](Self::rooted_at) pins) or at `<path>` when
///    given, relative paths resolved against the context dir.
/// 2. `memory://<name>[/<prefix>]`: an in-memory store, every handle on one
///    name sharing one backing for as long as any handle lives, so a store a
///    test seeds by name is the store the uri names.
/// 3. `s3://<bucket>[/<prefix>][?endpoint=<url>][&region=<region>]`: an
///    S3-compatible bucket, optionally rooted at a key prefix. With an
///    `endpoint` (eg a Cloudflare R2 account endpoint) the region defaults to
///    `auto`; without one an unnamed region is left to the AWS SDK's own default
///    provider chain.
///
///    The prefix is what lets a deploy give each version of a document its own
///    root: a binary baked with `s3://<bucket>/<deploy-id>` reads only the
///    document it shipped with, so the window between publishing a new document
///    and swapping the binary that serves it is not a window where the old
///    binary reads the new document.
/// 4. `dynamo://<table>[/<prefix>][?region=<region>]`: a DynamoDB table, the
///    table-native store a declaration wanting indexed queries names.
/// 5. `local-storage://<store>[/<prefix>]` / `indexed-db://<db>[/<prefix>]`:
///    browser storage (wasm), one named store or database per uri.
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
	/// A filesystem store. `path` overrides the context dir, else the store
	/// roots at the context dir itself.
	Fs {
		/// The store root, absolute or relative to the context dir.
		path: Option<SmolStr>,
	},
	/// An in-memory store, one backing per name for as long as any handle on
	/// it lives.
	Memory {
		/// The backing name.
		name: SmolStr,
		/// A key prefix the store roots at.
		prefix: Option<SmolStr>,
	},
	/// An S3-compatible bucket, the store a deployed task serves from.
	S3 {
		/// The bucket name.
		bucket: SmolStr,
		/// A key prefix the store roots at, so one bucket holds many
		/// independently addressed roots (a per-deploy document version).
		prefix: Option<SmolStr>,
		/// An S3-compatible service endpoint, eg
		/// `https://<account>.r2.cloudflarestorage.com`. Selects region `auto`
		/// by default, so one binary serves identically on AWS S3 and R2.
		endpoint: Option<SmolStr>,
		/// The bucket region, resolved at construction when absent.
		region: Option<SmolStr>,
	},
	/// A DynamoDB table.
	Dynamo {
		/// The table name.
		table: SmolStr,
		/// A key prefix the store roots at.
		prefix: Option<SmolStr>,
		/// The table region, resolved at construction when absent.
		region: Option<SmolStr>,
	},
	/// Browser `localStorage` (wasm).
	LocalStorage {
		/// The store name every key is prefixed with, so one origin holds many
		/// stores.
		store: SmolStr,
		/// A key prefix the store roots at.
		prefix: Option<SmolStr>,
	},
	/// Browser IndexedDB (wasm).
	IndexedDb {
		/// The database name, one database per store.
		db: SmolStr,
		/// A key prefix the store roots at.
		prefix: Option<SmolStr>,
	},
}

impl Default for StoreUri {
	fn default() -> Self { Self::Fs { path: None } }
}

/// The scheme of every scoped kind, ie `s3` in `s3://<bucket>`.
const MEMORY_SCHEME: &str = "memory";
const S3_SCHEME: &str = "s3";
const DYNAMO_SCHEME: &str = "dynamo";
const LOCAL_STORAGE_SCHEME: &str = "local-storage";
const INDEXED_DB_SCHEME: &str = "indexed-db";

impl StoreUri {
	/// Parse a store uri, erroring with the supported list on an unknown kind.
	pub fn parse(value: &str) -> Result<Self> {
		let value = value.trim();
		if let Some((scheme, rest)) = value.split_once("://") {
			return Self::parse_scoped(scheme, rest);
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
			other => bevybail!(
				"unknown store `{other}`, supported kinds: fs, fs:<path>, \
				memory://<name>[/<prefix>], \
				s3://<bucket>[/<prefix>][?endpoint=..][&region=..], \
				dynamo://<table>[/<prefix>][?region=..], \
				local-storage://<store>[/<prefix>] (wasm), \
				indexed-db://<db>[/<prefix>] (wasm)"
			),
		}
		.xok()
	}

	/// Parse a `<scheme>://<name>[/<prefix>][?<params>]` uri, the shape every
	/// self-rooted kind shares.
	fn parse_scoped(scheme: &str, rest: &str) -> Result<Self> {
		match scheme {
			MEMORY_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::Memory {
					name: tail.name,
					prefix: tail.prefix,
				}
			}
			S3_SCHEME => {
				let tail =
					ScopedTail::parse(scheme, rest, &["endpoint", "region"])?;
				Self::S3 {
					endpoint: tail.param("endpoint"),
					region: tail.param("region"),
					bucket: tail.name,
					prefix: tail.prefix,
				}
			}
			DYNAMO_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &["region"])?;
				Self::Dynamo {
					region: tail.param("region"),
					table: tail.name,
					prefix: tail.prefix,
				}
			}
			LOCAL_STORAGE_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::LocalStorage {
					store: tail.name,
					prefix: tail.prefix,
				}
			}
			INDEXED_DB_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::IndexedDb {
					db: tail.name,
					prefix: tail.prefix,
				}
			}
			other => bevybail!(
				"unknown store scheme `{other}://`, supported: memory, s3, \
				dynamo, local-storage, indexed-db"
			),
		}
		.xok()
	}

	/// Whether this store roots itself (a named memory store, a bucket, a table
	/// or browser storage), needing no local directory or filesystem walk. For
	/// these an entry name addresses the document *within* the store and there
	/// is no live-reload watch dir.
	pub fn is_self_rooted(&self) -> bool { !matches!(self, Self::Fs { .. }) }

	/// This store rooted at `subdir` below its current root, the uri form of
	/// `BlobStore::with_subdir`: a filesystem store joins the path, every
	/// other kind nests its key prefix.
	pub fn with_subdir(&self, subdir: impl AsRef<str>) -> Self {
		let subdir = subdir.as_ref().trim_matches('/');
		let join = |root: Option<&SmolStr>| -> Option<SmolStr> {
			match root {
				Some(root) => {
					format!("{}/{subdir}", root.trim_end_matches('/'))
				}
				None => subdir.to_string(),
			}
			.xmap(SmolStr::from)
			.xsome()
		};
		match self {
			Self::Fs { path } => Self::Fs {
				path: join(path.as_ref()),
			},
			Self::Memory { name, prefix } => Self::Memory {
				name: name.clone(),
				prefix: join(prefix.as_ref()),
			},
			Self::S3 {
				bucket,
				prefix,
				endpoint,
				region,
			} => Self::S3 {
				bucket: bucket.clone(),
				prefix: join(prefix.as_ref()),
				endpoint: endpoint.clone(),
				region: region.clone(),
			},
			Self::Dynamo {
				table,
				prefix,
				region,
			} => Self::Dynamo {
				table: table.clone(),
				prefix: join(prefix.as_ref()),
				region: region.clone(),
			},
			Self::LocalStorage { store, prefix } => Self::LocalStorage {
				store: store.clone(),
				prefix: join(prefix.as_ref()),
			},
			Self::IndexedDb { db, prefix } => Self::IndexedDb {
				db: db.clone(),
				prefix: join(prefix.as_ref()),
			},
		}
	}

	/// This uri with its filesystem root pinned to `dir`: a bare `fs` roots at
	/// `dir`, a relative `fs:<path>` at `<path>` under it, an absolute one
	/// stands alone, and every self-rooted kind is untouched. The entry
	/// resolver calls this with the resolved entry directory, so a `--repo=fs`
	/// means "the entry's own directory" rather than the cwd.
	pub fn rooted_at(&self, dir: &AbsPath) -> Self {
		match self {
			Self::Fs { path: None } => Self::Fs {
				path: Some(dir.to_string().into()),
			},
			Self::Fs { path: Some(path) } => Self::Fs {
				path: Some(match path.starts_with('/') {
					true => path.clone(),
					false => dir.join(path).to_string().into(),
				}),
			},
			other => other.clone(),
		}
	}
}

/// The parsed `<name>[/<prefix>][?<key>=<value>[&..]]` tail of a scoped uri.
struct ScopedTail {
	name: SmolStr,
	prefix: Option<SmolStr>,
	params: Vec<(SmolStr, SmolStr)>,
}

impl ScopedTail {
	/// Parse `rest`, accepting only the query `params` the kind declares. The
	/// name is the first segment; everything after the first `/` is the prefix
	/// the store roots at, with any trailing slash trimmed so `s3://b/p` and
	/// `s3://b/p/` are the one value.
	fn parse(scheme: &str, rest: &str, allowed: &[&str]) -> Result<Self> {
		let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
		let (name, prefix) = match path.split_once('/') {
			Some((name, prefix)) => {
				let prefix = prefix.trim_matches('/');
				(name, (!prefix.is_empty()).then(|| prefix.into()))
			}
			None => (path, None),
		};
		if name.is_empty() {
			bevybail!("store `{scheme}://` is missing a name");
		}
		let mut params = Vec::new();
		for pair in query.split('&').filter(|pair| !pair.is_empty()) {
			match pair.split_once('=') {
				Some((key, value)) if allowed.contains(&key) => {
					params.push((key.into(), value.into()));
				}
				_ => bevybail!(
					"unknown {scheme} store query param `{pair}`, supported: {}",
					match allowed.is_empty() {
						true => "none".to_string(),
						false => allowed.join(", "),
					}
				),
			}
		}
		Self {
			name: name.into(),
			prefix,
			params,
		}
		.xok()
	}

	/// The value of query param `key`, if given.
	fn param(&self, key: &str) -> Option<SmolStr> {
		self.params
			.iter()
			.find(|(name, _)| name == key)
			.map(|(_, value)| value.clone())
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
			Self::Memory { name, prefix } => {
				fmt_scoped(f, MEMORY_SCHEME, name, prefix, &[])
			}
			Self::S3 {
				bucket,
				prefix,
				endpoint,
				region,
			} => fmt_scoped(f, S3_SCHEME, bucket, prefix, &[
				("endpoint", endpoint),
				("region", region),
			]),
			Self::Dynamo {
				table,
				prefix,
				region,
			} => fmt_scoped(f, DYNAMO_SCHEME, table, prefix, &[(
				"region", region,
			)]),
			Self::LocalStorage { store, prefix } => {
				fmt_scoped(f, LOCAL_STORAGE_SCHEME, store, prefix, &[])
			}
			Self::IndexedDb { db, prefix } => {
				fmt_scoped(f, INDEXED_DB_SCHEME, db, prefix, &[])
			}
		}
	}
}

/// Render `<scheme>://<name>[/<prefix>][?<key>=<value>[&..]]`, the query
/// separator `?` for the first present param and `&` after, so the rendered
/// uri parses back into the same value.
fn fmt_scoped(
	f: &mut fmt::Formatter<'_>,
	scheme: &str,
	name: &str,
	prefix: &Option<SmolStr>,
	params: &[(&str, &Option<SmolStr>)],
) -> fmt::Result {
	write!(f, "{scheme}://{name}")?;
	if let Some(prefix) = prefix {
		write!(f, "/{prefix}")?;
	}
	let mut sep = '?';
	for (key, value) in params {
		if let Some(value) = value {
			write!(f, "{sep}{key}={value}")?;
			sep = '&';
		}
	}
	Ok(())
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
			"s3://my-site/01a084cb-6e31-7a53-92ae-67c282615871",
			"s3://my-site/versions/2?region=us-east-1",
			"memory://fixtures",
			"memory://fixtures/docs",
			"local-storage://beet",
			"local-storage://beet/analytics",
			"indexed-db://beet",
			"indexed-db://beet-site--dev--analytics/v1",
			"dynamo://beet-site--prod--analytics",
			"dynamo://beet-site--prod--analytics/v1?region=us-west-2",
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
				prefix: None,
				endpoint: Some("http://e".into()),
				region: Some("r".into()),
			});
		StoreUri::parse("dynamo://t/p?region=r").unwrap().xpect_eq(
			StoreUri::Dynamo {
				table: "t".into(),
				prefix: Some("p".into()),
				region: Some("r".into()),
			},
		);
		StoreUri::parse("indexed-db://d/p").unwrap().xpect_eq(
			StoreUri::IndexedDb {
				db: "d".into(),
				prefix: Some("p".into()),
			},
		);
		StoreUri::parse("local-storage://s").unwrap().xpect_eq(
			StoreUri::LocalStorage {
				store: "s".into(),
				prefix: None,
			},
		);
		StoreUri::parse("memory://m/p")
			.unwrap()
			.xpect_eq(StoreUri::Memory {
				name: "m".into(),
				prefix: Some("p".into()),
			});
	}

	/// Only a filesystem store needs its context dir; a named memory store, a
	/// bucket, a table and browser storage root themselves.
	#[crate::test]
	fn self_rooted_kinds() {
		for uri in [
			"memory://m",
			"s3://b",
			"dynamo://t",
			"local-storage://s",
			"indexed-db://d",
		] {
			StoreUri::parse(uri).unwrap().is_self_rooted().xpect_true();
		}
		for uri in ["fs", "fs:site"] {
			StoreUri::parse(uri).unwrap().is_self_rooted().xpect_false();
		}
	}

	#[crate::test]
	fn rejects_malformed() {
		StoreUri::parse("nope")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown store");
		// a memory store is always named
		StoreUri::parse("memory")
			.unwrap_err()
			.to_string()
			.xpect_contains("memory://<name>");
		StoreUri::parse("memory://")
			.unwrap_err()
			.to_string()
			.xpect_contains("missing a name");
		StoreUri::parse("nope://x")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown store scheme");
		StoreUri::parse("s3://")
			.unwrap_err()
			.to_string()
			.xpect_contains("missing a name");
		StoreUri::parse("indexed-db://")
			.unwrap_err()
			.to_string()
			.xpect_contains("missing a name");
		StoreUri::parse("s3://b?nope=1")
			.unwrap_err()
			.to_string()
			.xpect_contains("unknown s3 store query param");
		// a kind with no params refuses every one
		StoreUri::parse("indexed-db://d?region=r")
			.unwrap_err()
			.to_string()
			.xpect_contains("supported: none");
	}

	/// A subdir nests below whatever root the uri already has, uniformly
	/// across every kind.
	#[crate::test]
	fn with_subdir_nests_below_the_root() {
		let nested = |uri: &str, subdir: &str| {
			StoreUri::parse(uri)
				.unwrap()
				.with_subdir(subdir)
				.to_string()
		};
		nested("memory://m", "v1").xpect_eq("memory://m/v1");
		nested("s3://site", "v1").xpect_eq("s3://site/v1");
		nested("s3://site/docs?region=us-east-1", "/v1/")
			.xpect_eq("s3://site/docs/v1?region=us-east-1");
		nested("fs", "v1").xpect_eq("fs:v1");
		nested("fs:../site", "v1").xpect_eq("fs:../site/v1");
		nested("dynamo://t?region=r", "v1").xpect_eq("dynamo://t/v1?region=r");
		nested("indexed-db://d", "analytics")
			.xpect_eq("indexed-db://d/analytics");
		nested("local-storage://s/a", "b").xpect_eq("local-storage://s/a/b");
	}

	/// Pinning a context dir resolves the filesystem kinds and leaves every
	/// self-rooted kind alone.
	#[crate::test]
	fn rooted_at_pins_the_filesystem_root() {
		let dir = AbsPath::new_unchecked("/srv");
		let rooted = |uri: &str| {
			StoreUri::parse(uri).unwrap().rooted_at(&dir).to_string()
		};
		rooted("fs").xpect_eq("fs:/srv");
		rooted("fs:site").xpect_eq("fs:/srv/site");
		rooted("fs:/data").xpect_eq("fs:/data");
		rooted("s3://b").xpect_eq("s3://b");
		rooted("memory://m").xpect_eq("memory://m");
	}

	#[crate::test]
	fn a_prefix_roots_the_store() {
		let StoreUri::S3 { bucket, prefix, .. } =
			StoreUri::parse("s3://my-site/deploy-1/").unwrap()
		else {
			panic!("expected an s3 store");
		};
		bucket.as_str().xpect_eq("my-site");
		prefix.unwrap().as_str().xpect_eq("deploy-1");
		// ..and a bucket with no prefix keeps naming the bucket root
		let StoreUri::S3 { prefix, .. } =
			StoreUri::parse("s3://my-site?region=us-east-1").unwrap()
		else {
			panic!("expected an s3 store");
		};
		prefix.xpect_none();
	}
}
