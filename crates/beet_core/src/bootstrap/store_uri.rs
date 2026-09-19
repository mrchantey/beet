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
/// bucket, a table, a browser database) plus the root it starts at, so the one
/// uri is the whole answer and nothing downstream patches it. Every kind
/// answers the same two questions, [`name`](Self::name) (the backing) and
/// [`path_prefix`](Self::path_prefix) (the root within it), so no consumer
/// needs a per-kind vocabulary. The grammar round-trips through [`Display`]:
///
/// 1. `fs` / `fs:<path>`: a filesystem store, rooted at the context dir (the
///    cwd, or the dir [`rooted_at`](Self::rooted_at) pins) or at `<path>` when
///    given, relative paths resolved against the context dir. A leading `~`
///    (`fs:~`, `fs:~/<path>`) expands to the home directory at parse, so the
///    rendered uri is the absolute path.
/// 2. `memory://<name>[/<path_prefix>]`: an in-memory store, every handle on one
///    name sharing one backing for as long as any handle lives, so a store a
///    test seeds by name is the store the uri names.
/// 3. `s3://<bucket>[/<path_prefix>][?endpoint=<url>][&region=<region>]`: an
///    S3-compatible bucket, optionally rooted at a key prefix. With an
///    `endpoint` (eg a Cloudflare R2 account endpoint) the region defaults to
///    `auto`; without one an unnamed region is left to the AWS SDK's own default
///    provider chain.
///
///    The path prefix is what lets a deploy give each version of a document
///    its own root: a binary baked with `s3://<bucket>/<deploy-id>` reads only the
///    document it shipped with, so the window between publishing a new document
///    and swapping the binary that serves it is not a window where the old
///    binary reads the new document.
/// 4. `dynamo://<table>[/<path_prefix>][?region=<region>]`: a DynamoDB table, the
///    table-native store a declaration wanting indexed queries names.
/// 5. `local-storage://<store>[/<path_prefix>]` / `indexed-db://<db>[/<path_prefix>]`:
///    browser storage (wasm), one named store or database per uri.
/// 6. `r2://<binding>[/<path_prefix>]`: a Cloudflare R2 bucket reached through the
///    Worker's binding of that name (a Worker), not the S3-compatible API,
///    which is an `s3://<bucket>?endpoint=..` uri.
/// 7. `http://<host>[:port][/<path_prefix>]` / `https://..` / `http:[<path_prefix>]`:
///    a store published over http (a site's `<ServeBlobs prefix="repo"/>`),
///    read-only. The scheme-only form is origin-relative, the `fs:<path>` of
///    the web: a browser reads it against `location.origin`, a native process
///    against its canonical loopback server. A process forks a remote repo
///    into a local store (`--store-fork`) rather than writing to it.
/// 8. `sqlite:<path>`: a SQLite database file, the table-native store a local
///    index names. Relative paths resolve against the context dir and `~`
///    expands exactly as for `fs:`. The file is a whole store: there is no key
///    prefix within it.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let uri = StoreUri::parse("s3://my-site/docs?region=us-east-1").unwrap();
/// uri.name().xpect_eq(Some("my-site"));
/// uri.path_prefix().unwrap().as_str().xpect_eq("docs");
/// uri.to_string().xpect_eq("s3://my-site/docs?region=us-east-1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StoreUri {
	/// A filesystem store. `path_prefix` overrides the context dir, else the
	/// store roots at the context dir itself.
	Fs {
		/// The store root, absolute or relative to the context dir.
		path_prefix: Option<SmolPath>,
	},
	/// An in-memory store, one backing per name for as long as any handle on
	/// it lives.
	Memory {
		/// The backing name.
		name: SmolStr,
		/// The key prefix the store roots at.
		path_prefix: Option<RelPath>,
	},
	/// An S3-compatible bucket, the store a deployed task serves from.
	S3 {
		/// The bucket name.
		name: SmolStr,
		/// The key prefix the store roots at, so one bucket holds many
		/// independently addressed roots (a per-deploy document version).
		path_prefix: Option<RelPath>,
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
		name: SmolStr,
		/// The key prefix the store roots at.
		path_prefix: Option<RelPath>,
		/// The table region, resolved at construction when absent.
		region: Option<SmolStr>,
	},
	/// Browser `localStorage` (wasm).
	LocalStorage {
		/// The store name every key is prefixed with, so one origin holds many
		/// stores.
		name: SmolStr,
		/// The key prefix the store roots at.
		path_prefix: Option<RelPath>,
	},
	/// Browser IndexedDB (wasm).
	IndexedDb {
		/// The database name, one database per store.
		name: SmolStr,
		/// The key prefix the store roots at.
		path_prefix: Option<RelPath>,
	},
	/// A Cloudflare R2 bucket through a Worker's binding.
	R2 {
		/// The binding name, as declared in the Worker's `wrangler.toml`.
		name: SmolStr,
		/// The key prefix the store roots at.
		path_prefix: Option<RelPath>,
	},
	/// A store published over http, read-only.
	Http {
		/// The origin it is served from, ie `https://beet.org`; `None` for the
		/// page's own origin in a browser, the canonical loopback server
		/// natively.
		origin: Option<SmolStr>,
		/// The url path prefix the store roots at.
		path_prefix: Option<RelPath>,
	},
	/// A SQLite database file, a whole store with no key prefix.
	Sqlite {
		/// The database file, absolute or relative to the context dir.
		path: SmolPath,
	},
}

impl Default for StoreUri {
	fn default() -> Self { Self::Fs { path_prefix: None } }
}

/// The scheme of every scoped kind, ie `s3` in `s3://<bucket>`.
const MEMORY_SCHEME: &str = "memory";
const S3_SCHEME: &str = "s3";
const DYNAMO_SCHEME: &str = "dynamo";
const LOCAL_STORAGE_SCHEME: &str = "local-storage";
const INDEXED_DB_SCHEME: &str = "indexed-db";
const R2_SCHEME: &str = "r2";
const HTTP_SCHEME: &str = "http";
const HTTPS_SCHEME: &str = "https";

impl StoreUri {
	/// Parse a store uri, erroring with the supported list on an unknown kind.
	pub fn parse(value: &str) -> Result<Self> {
		let value = value.trim();
		if let Some((scheme, rest)) = value.split_once("://") {
			return Self::parse_scoped(scheme, rest);
		}
		// `fs:` and `fs:.` are the context dir itself, ie a bare `fs`
		if let Some(path) = value.strip_prefix("fs:") {
			return Self::Fs {
				path_prefix: Self::expand_home(path.trim())?
					.xmap(SmolPath::new)
					.xmap(non_empty),
			}
			.xok();
		}
		// a database file is named or nothing: `sqlite:` alone is an error
		if let Some(path) = value.strip_prefix("sqlite:") {
			let path = Self::expand_home(path.trim())?.xmap(SmolPath::new);
			if path.as_str().is_empty() {
				bevybail!("store `sqlite:` is missing a database path");
			}
			return Self::Sqlite { path }.xok();
		}
		// `http:` and `http:/` are the origin itself, ie a bare `http`
		if let Some(path) = value.strip_prefix("http:") {
			return Self::Http {
				origin: None,
				path_prefix: RelPath::new(path.trim()).xmap(non_empty),
			}
			.xok();
		}
		match value {
			"fs" => Self::Fs { path_prefix: None },
			"http" => Self::Http {
				origin: None,
				path_prefix: None,
			},
			other => bevybail!(
				"unknown store `{other}`, supported kinds: fs, fs:<path>, \
				memory://<name>[/<path_prefix>], \
				s3://<bucket>[/<path_prefix>][?endpoint=..][&region=..], \
				dynamo://<table>[/<path_prefix>][?region=..], \
				local-storage://<store>[/<path_prefix>] (wasm), \
				indexed-db://<db>[/<path_prefix>] (wasm), \
				r2://<binding>[/<path_prefix>] (cloudflare worker), \
				http://<host>[/<path_prefix>], https://.., http:<path_prefix>, \
				sqlite:<path>"
			),
		}
		.xok()
	}

	/// Expand a leading `~` (exactly `~`, or `~/..`) to the home directory, so
	/// a uri never spells a user's home; any other `~foo` is a literal segment.
	fn expand_home(path: &str) -> Result<SmolStr> {
		let rest = match path {
			"~" => "",
			path if path.starts_with("~/") => &path[1..],
			path => return SmolStr::new(path).xok(),
		};
		match env_ext::var("HOME") {
			Ok(home) => SmolStr::from(format!("{home}{rest}")).xok(),
			Err(_) => bevybail!(
				"`~` in store path `{path}` needs a HOME environment variable"
			),
		}
	}

	/// The http store `url` names: its authority is the origin (absent, the
	/// page's own) and its path the prefix. How a page spells the repo its
	/// browser process reads, `<Wasm repo="/repo"/>`.
	pub fn http(url: &Url) -> Self {
		Self::Http {
			origin: url.authority().map(|authority| {
				let scheme = match url.scheme() {
					Scheme::None => HTTP_SCHEME,
					Scheme::Https => HTTPS_SCHEME,
					_ => HTTP_SCHEME,
				};
				format!("{scheme}://{authority}").into()
			}),
			path_prefix: RelPath::from_segments(url.path()).xmap(non_empty),
		}
	}

	/// Parse a `<scheme>://<name>[/<path_prefix>][?<params>]` uri, the shape every
	/// self-rooted kind shares.
	fn parse_scoped(scheme: &str, rest: &str) -> Result<Self> {
		match scheme {
			MEMORY_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::Memory {
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			S3_SCHEME => {
				let tail =
					ScopedTail::parse(scheme, rest, &["endpoint", "region"])?;
				Self::S3 {
					endpoint: tail.param("endpoint"),
					region: tail.param("region"),
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			DYNAMO_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &["region"])?;
				Self::Dynamo {
					region: tail.param("region"),
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			LOCAL_STORAGE_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::LocalStorage {
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			INDEXED_DB_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::IndexedDb {
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			R2_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::R2 {
					name: tail.name,
					path_prefix: tail.path_prefix,
				}
			}
			HTTP_SCHEME | HTTPS_SCHEME => {
				let tail = ScopedTail::parse(scheme, rest, &[])?;
				Self::Http {
					origin: Some(format!("{scheme}://{}", tail.name).into()),
					path_prefix: tail.path_prefix,
				}
			}
			other => bevybail!(
				"unknown store scheme `{other}://`, supported: memory, s3, \
				dynamo, local-storage, indexed-db, r2, http, https"
			),
		}
		.xok()
	}

	/// The backing this uri names: a memory backing, a bucket, a table, a
	/// browser database, a Worker binding, an http origin, a database file.
	/// `None` for a filesystem store, whose root is its
	/// [`path_prefix`](Self::path_prefix), and for an origin-relative http
	/// store, whose origin is the context's.
	pub fn name(&self) -> Option<&str> {
		match self {
			Self::Fs { .. } => None,
			Self::Memory { name, .. }
			| Self::S3 { name, .. }
			| Self::Dynamo { name, .. }
			| Self::LocalStorage { name, .. }
			| Self::IndexedDb { name, .. }
			| Self::R2 { name, .. } => Some(name.as_str()),
			Self::Http { origin, .. } => origin.as_deref(),
			Self::Sqlite { path } => Some(path.as_str()),
		}
	}

	/// The root within the backing, the prefix every key resolves under: a
	/// filesystem path for `fs` (absolute, or relative to the context dir), a
	/// key prefix for every other kind, `None` for a whole-store kind
	/// (`sqlite:`).
	pub fn path_prefix(&self) -> Option<&SmolPath> {
		match self {
			Self::Fs { path_prefix } => path_prefix.as_ref(),
			Self::Memory { path_prefix, .. }
			| Self::S3 { path_prefix, .. }
			| Self::Dynamo { path_prefix, .. }
			| Self::LocalStorage { path_prefix, .. }
			| Self::IndexedDb { path_prefix, .. }
			| Self::R2 { path_prefix, .. }
			| Self::Http { path_prefix, .. } => path_prefix.as_deref(),
			// a database file is a whole store
			Self::Sqlite { .. } => None,
		}
	}

	/// Whether this store roots itself (a named memory store, a bucket, a table,
	/// browser storage or an http origin), needing no local directory or
	/// filesystem walk. For these an entry name addresses the document *within*
	/// the store and there is no live-reload watch dir.
	pub fn is_self_rooted(&self) -> bool { !matches!(self, Self::Fs { .. }) }

	/// Whether this store is read through the network rather than owned by the
	/// process: a process on such a repo forks its edits into a store fork
	/// (`--store-fork`) rather than writing back.
	pub fn is_remote(&self) -> bool { matches!(self, Self::Http { .. }) }

	/// The local-storage key under which a browser holding a fork of this
	/// store says so: one bit the fork writes on its first write and a served
	/// page's pre-boot script reads before the world exists, hiding the
	/// published page from a returning editor until their fork paints. Both
	/// sides spell the store as the launch does (`--repo`), so they agree by
	/// construction.
	pub fn fork_mark(&self) -> String { format!("beet:fork:{self}") }

	/// This store rooted at `subdir` below its current root, the uri form of
	/// `BlobStore::with_subdir`: every kind nests `subdir` under its path
	/// prefix. A `subdir` is a key, so a leading `/` or an escaping `..`
	/// cannot climb above the current root. A whole-store kind (`sqlite:`)
	/// has no prefix to nest under and is returned as is: scope its provider
	/// instead.
	pub fn with_subdir(&self, subdir: impl Into<RelPath>) -> Self {
		let subdir = subdir.into();
		let nest = |path_prefix: &Option<RelPath>| -> Option<RelPath> {
			match path_prefix {
				Some(path_prefix) => path_prefix.join(&subdir),
				None => subdir.clone(),
			}
			.xmap(non_empty)
		};
		match self {
			Self::Fs { path_prefix } => Self::Fs {
				path_prefix: match path_prefix {
					Some(path_prefix) => path_prefix.join(&subdir),
					None => subdir.clone().into_smol_path(),
				}
				.xmap(non_empty),
			},
			Self::Memory { name, path_prefix } => Self::Memory {
				name: name.clone(),
				path_prefix: nest(path_prefix),
			},
			Self::S3 {
				name,
				path_prefix,
				endpoint,
				region,
			} => Self::S3 {
				name: name.clone(),
				path_prefix: nest(path_prefix),
				endpoint: endpoint.clone(),
				region: region.clone(),
			},
			Self::Dynamo {
				name,
				path_prefix,
				region,
			} => Self::Dynamo {
				name: name.clone(),
				path_prefix: nest(path_prefix),
				region: region.clone(),
			},
			Self::LocalStorage { name, path_prefix } => Self::LocalStorage {
				name: name.clone(),
				path_prefix: nest(path_prefix),
			},
			Self::IndexedDb { name, path_prefix } => Self::IndexedDb {
				name: name.clone(),
				path_prefix: nest(path_prefix),
			},
			Self::R2 { name, path_prefix } => Self::R2 {
				name: name.clone(),
				path_prefix: nest(path_prefix),
			},
			Self::Http {
				origin,
				path_prefix,
			} => Self::Http {
				origin: origin.clone(),
				path_prefix: nest(path_prefix),
			},
			Self::Sqlite { .. } => self.clone(),
		}
	}

	/// This uri with its filesystem root pinned to `dir`: a bare `fs` roots at
	/// `dir`, a relative `fs:<path>` or `sqlite:<path>` at `<path>` under it,
	/// an absolute one stands alone, and every self-rooted kind is untouched.
	/// The entry resolver calls this with the resolved entry directory, so a
	/// `--repo=fs` means "the entry's own directory" rather than the cwd.
	pub fn rooted_at(&self, dir: &AbsPath) -> Self {
		let under_dir = |path: &SmolPath| match path.is_absolute() {
			true => path.clone(),
			false => dir.join(path).into_smol_path(),
		};
		match self {
			Self::Fs { path_prefix: None } => Self::Fs {
				path_prefix: Some(dir.as_smol_path().clone()),
			},
			Self::Fs {
				path_prefix: Some(path_prefix),
			} => Self::Fs {
				path_prefix: Some(under_dir(path_prefix)),
			},
			Self::Sqlite { path } => Self::Sqlite {
				path: under_dir(path),
			},
			other => other.clone(),
		}
	}
}

/// An empty path prefix is no root at all, so `fs:.` is `fs` and `s3://b/` is
/// `s3://b`, and the rendered uri parses back into the same value.
fn non_empty<T: AsRef<str>>(path: T) -> Option<T> {
	(!path.as_ref().is_empty()).then_some(path)
}

/// The parsed `<name>[/<path_prefix>][?<key>=<value>[&..]]` tail of a scoped
/// uri.
struct ScopedTail {
	name: SmolStr,
	path_prefix: Option<RelPath>,
	params: Vec<(SmolStr, SmolStr)>,
}

impl ScopedTail {
	/// Parse `rest`, accepting only the query `params` the kind declares. The
	/// name is the first segment; everything after the first `/` is the key
	/// the store roots at, a [`RelPath`] so `s3://b/p`, `s3://b/p/` and
	/// `s3://b//p` are the one value.
	fn parse(scheme: &str, rest: &str, allowed: &[&str]) -> Result<Self> {
		let (locator, query) = rest.split_once('?').unwrap_or((rest, ""));
		let (name, path_prefix) = match locator.split_once('/') {
			Some((name, prefix)) => {
				(name, RelPath::new(prefix).xmap(non_empty))
			}
			None => (locator, None),
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
			path_prefix,
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
			Self::Fs { path_prefix: None } => write!(f, "fs"),
			Self::Fs {
				path_prefix: Some(path_prefix),
			} => write!(f, "fs:{path_prefix}"),
			Self::Memory { name, path_prefix } => {
				fmt_scoped(f, MEMORY_SCHEME, name, path_prefix, &[])
			}
			Self::S3 {
				name,
				path_prefix,
				endpoint,
				region,
			} => fmt_scoped(f, S3_SCHEME, name, path_prefix, &[
				("endpoint", endpoint),
				("region", region),
			]),
			Self::Dynamo {
				name,
				path_prefix,
				region,
			} => fmt_scoped(f, DYNAMO_SCHEME, name, path_prefix, &[(
				"region", region,
			)]),
			Self::LocalStorage { name, path_prefix } => {
				fmt_scoped(f, LOCAL_STORAGE_SCHEME, name, path_prefix, &[])
			}
			Self::IndexedDb { name, path_prefix } => {
				fmt_scoped(f, INDEXED_DB_SCHEME, name, path_prefix, &[])
			}
			Self::R2 { name, path_prefix } => {
				fmt_scoped(f, R2_SCHEME, name, path_prefix, &[])
			}
			// an origin carries its own scheme, `https://beet.org`
			Self::Http {
				origin: Some(origin),
				path_prefix,
			} => match path_prefix {
				Some(path_prefix) => write!(f, "{origin}/{path_prefix}"),
				None => write!(f, "{origin}"),
			},
			Self::Http {
				origin: None,
				path_prefix,
			} => match path_prefix {
				Some(path_prefix) => write!(f, "http:{path_prefix}"),
				None => write!(f, "http"),
			},
			Self::Sqlite { path } => write!(f, "sqlite:{path}"),
		}
	}
}

/// Render `<scheme>://<name>[/<path_prefix>][?<key>=<value>[&..]]`, the query
/// separator `?` for the first present param and `&` after, so the rendered
/// uri parses back into the same value.
fn fmt_scoped(
	f: &mut fmt::Formatter<'_>,
	scheme: &str,
	name: &str,
	path_prefix: &Option<RelPath>,
	params: &[(&str, &Option<SmolStr>)],
) -> fmt::Result {
	write!(f, "{scheme}://{name}")?;
	if let Some(path_prefix) = path_prefix {
		write!(f, "/{path_prefix}")?;
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
			"r2://SITE_BUCKET",
			"r2://SITE_BUCKET/v1",
			"s3://my-bucket",
			"s3://my-bucket?region=us-east-1",
			"s3://my-bucket?endpoint=https://acc.r2.cloudflarestorage.com",
			"s3://my-bucket?endpoint=https://acc.r2.cloudflarestorage.com&region=auto",
			"http",
			"http:repo",
			"http:examples/wasm",
			"http://127.0.0.1:8337/repo",
			"https://beet.org",
			"https://beet.org/repo",
			"sqlite:data.db",
			"sqlite:../index/data.db",
			"sqlite:/var/lib/beet/data.db",
		] {
			StoreUri::parse(uri).unwrap().to_string().xpect_eq(uri);
		}
	}

	#[crate::test]
	fn parses_fields() {
		StoreUri::parse("fs")
			.unwrap()
			.xpect_eq(StoreUri::Fs { path_prefix: None });
		StoreUri::parse("fs:site").unwrap().xpect_eq(StoreUri::Fs {
			path_prefix: Some("site".into()),
		});
		StoreUri::parse("s3://b?endpoint=http://e&region=r")
			.unwrap()
			.xpect_eq(StoreUri::S3 {
				name: "b".into(),
				path_prefix: None,
				endpoint: Some("http://e".into()),
				region: Some("r".into()),
			});
		StoreUri::parse("dynamo://t/p?region=r").unwrap().xpect_eq(
			StoreUri::Dynamo {
				name: "t".into(),
				path_prefix: Some("p".into()),
				region: Some("r".into()),
			},
		);
		StoreUri::parse("indexed-db://d/p").unwrap().xpect_eq(
			StoreUri::IndexedDb {
				name: "d".into(),
				path_prefix: Some("p".into()),
			},
		);
		StoreUri::parse("local-storage://s").unwrap().xpect_eq(
			StoreUri::LocalStorage {
				name: "s".into(),
				path_prefix: None,
			},
		);
		StoreUri::parse("memory://m/p")
			.unwrap()
			.xpect_eq(StoreUri::Memory {
				name: "m".into(),
				path_prefix: Some("p".into()),
			});
		StoreUri::parse("r2://SITE_BUCKET/p")
			.unwrap()
			.xpect_eq(StoreUri::R2 {
				name: "SITE_BUCKET".into(),
				path_prefix: Some("p".into()),
			});
		StoreUri::parse("https://beet.org/repo").unwrap().xpect_eq(
			StoreUri::Http {
				origin: Some("https://beet.org".into()),
				path_prefix: Some("repo".into()),
			},
		);
		// the origin-relative spellings all root at the page's own origin
		for uri in ["http", "http:", "http:/"] {
			StoreUri::parse(uri).unwrap().xpect_eq(StoreUri::Http {
				origin: None,
				path_prefix: None,
			});
		}
		StoreUri::parse("http:/repo")
			.unwrap()
			.xpect_eq(StoreUri::Http {
				origin: None,
				path_prefix: Some("repo".into()),
			});
		StoreUri::parse("sqlite:data.db")
			.unwrap()
			.xpect_eq(StoreUri::Sqlite {
				path: "data.db".into(),
			});
	}

	/// A leading `~` is the home directory, expanded at parse so the rendered
	/// uri is absolute; `~foo` is a literal dir name.
	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn expands_home_in_fs_paths() {
		let home = env_ext::var("HOME").unwrap();
		StoreUri::parse("fs:~/site")
			.unwrap()
			.to_string()
			.xpect_eq(format!("fs:{home}/site"));
		StoreUri::parse("fs:~")
			.unwrap()
			.to_string()
			.xpect_eq(format!("fs:{home}"));
		StoreUri::parse("fs:~foo")
			.unwrap()
			.to_string()
			.xpect_eq("fs:~foo");
		StoreUri::parse("sqlite:~/data.db")
			.unwrap()
			.to_string()
			.xpect_eq(format!("sqlite:{home}/data.db"));
	}

	/// A page names its repo as a url: an authority is the origin, a bare
	/// path the page's own.
	#[crate::test]
	fn http_from_url() {
		StoreUri::http(&Url::coerce("/examples/wasm"))
			.to_string()
			.xpect_eq("http:examples/wasm");
		StoreUri::http(&Url::coerce("https://beet.org/repo/"))
			.to_string()
			.xpect_eq("https://beet.org/repo");
		StoreUri::http(&Url::coerce("/"))
			.to_string()
			.xpect_eq("http");
	}

	/// Every kind answers `name()`/`path_prefix()` uniformly: a filesystem
	/// store has no name and its root is its path prefix, every other kind
	/// names its backing and roots at a key prefix.
	#[crate::test]
	fn name_and_path_prefix() {
		let parts = |uri: &str| {
			let uri = StoreUri::parse(uri).unwrap();
			(
				uri.name().map(|name| name.to_string()),
				uri.path_prefix().map(|prefix| prefix.to_string()),
			)
		};
		parts("fs").xpect_eq((None, None));
		parts("fs:../site").xpect_eq((None, Some("../site".into())));
		parts("fs:/abs/site").xpect_eq((None, Some("/abs/site".into())));
		parts("memory://m").xpect_eq((Some("m".into()), None));
		parts("s3://b/p?region=r")
			.xpect_eq((Some("b".into()), Some("p".into())));
		parts("dynamo://t/a/b")
			.xpect_eq((Some("t".into()), Some("a/b".into())));
		parts("local-storage://s/p")
			.xpect_eq((Some("s".into()), Some("p".into())));
		parts("indexed-db://d").xpect_eq((Some("d".into()), None));
		parts("r2://B/p").xpect_eq((Some("B".into()), Some("p".into())));
		parts("https://beet.org/repo")
			.xpect_eq((Some("https://beet.org".into()), Some("repo".into())));
		parts("http:repo").xpect_eq((None, Some("repo".into())));
		// a database file is the backing and a whole store
		parts("sqlite:data.db").xpect_eq((Some("data.db".into()), None));
	}

	/// A scoped path prefix is a key: the text is cleaned, a leading or
	/// repeated `/` is dropped and an escaping `..` cannot climb above the
	/// backing. A filesystem prefix keeps its text, root and `..` included.
	#[crate::test]
	fn path_prefixes_are_keys() {
		let prefix = |uri: &str| {
			StoreUri::parse(uri)
				.unwrap()
				.path_prefix()
				.map(|prefix| prefix.to_string())
		};
		prefix("s3://b//p/").xpect_eq(Some("p".into()));
		prefix("s3://b/./p/../q").xpect_eq(Some("q".into()));
		prefix("s3://b/../p").xpect_eq(Some("p".into()));
		prefix("s3://b/").xpect_none();
		prefix("memory://m/.").xpect_none();
		prefix("fs:.").xpect_none();
		prefix("fs:./a//b/").xpect_eq(Some("a/b".into()));
		prefix("fs:../a").xpect_eq(Some("../a".into()));
	}

	/// Only a filesystem store needs its context dir; a named memory store, a
	/// bucket, a table, browser storage and a Worker binding root themselves.
	#[crate::test]
	fn self_rooted_kinds() {
		for uri in [
			"memory://m",
			"s3://b",
			"dynamo://t",
			"local-storage://s",
			"indexed-db://d",
			"r2://B",
			"http:repo",
			"https://beet.org/repo",
			"sqlite:data.db",
		] {
			StoreUri::parse(uri).unwrap().is_self_rooted().xpect_true();
		}
		for uri in ["fs", "fs:site"] {
			StoreUri::parse(uri).unwrap().is_self_rooted().xpect_false();
		}
		// only the http kinds are remote
		StoreUri::parse("http:repo")
			.unwrap()
			.is_remote()
			.xpect_true();
		StoreUri::parse("s3://b").unwrap().is_remote().xpect_false();
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
		// a database file is always named
		StoreUri::parse("sqlite:")
			.unwrap_err()
			.to_string()
			.xpect_contains("missing a database path");
	}

	/// A subdir nests below whatever root the uri already has, uniformly
	/// across every kind, and cannot climb above it.
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
		nested("s3://site/docs", "../v1").xpect_eq("s3://site/docs/v1");
		nested("s3://site", "").xpect_eq("s3://site");
		nested("fs", "v1").xpect_eq("fs:v1");
		nested("fs:../site", "v1").xpect_eq("fs:../site/v1");
		nested("fs:/srv", "/v1").xpect_eq("fs:/srv/v1");
		nested("dynamo://t?region=r", "v1").xpect_eq("dynamo://t/v1?region=r");
		nested("indexed-db://d", "analytics")
			.xpect_eq("indexed-db://d/analytics");
		nested("local-storage://s/a", "b").xpect_eq("local-storage://s/a/b");
		nested("r2://B", "v1").xpect_eq("r2://B/v1");
		nested("http", "repo").xpect_eq("http:repo");
		nested("https://beet.org/repo", "v1")
			.xpect_eq("https://beet.org/repo/v1");
		// a whole store has no prefix to nest under
		nested("sqlite:data.db", "v1").xpect_eq("sqlite:data.db");
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
		rooted("fs:../other").xpect_eq("fs:/other");
		rooted("fs:/data").xpect_eq("fs:/data");
		rooted("sqlite:data.db").xpect_eq("sqlite:/srv/data.db");
		rooted("sqlite:/var/data.db").xpect_eq("sqlite:/var/data.db");
		rooted("s3://b").xpect_eq("s3://b");
		rooted("memory://m").xpect_eq("memory://m");
		rooted("r2://B/p").xpect_eq("r2://B/p");
		rooted("http:repo").xpect_eq("http:repo");
	}
}
