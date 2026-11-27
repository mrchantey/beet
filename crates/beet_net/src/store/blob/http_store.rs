//! [`HttpStore`]: a store read over http, the client half of a
//! `<ServeBlobs>` mount.

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A store published over http, read-only: a site's package store as its
/// `<ServeBlobs prefix="package"/>` serves it, read by the browser process the
/// site's page boots and by a native process pulling the site into a terminal
/// (`beet --package=https://beet.org/package --server=tui`).
///
/// Every key is fetched at `{base}/{key}` through [`Request::send`], so an
/// origin-relative base (`http:package`) resolves against the page's origin in a
/// browser and the canonical loopback server natively. A miss is the mount's
/// 404, so a consumer distinguishes an absent file from a broken store.
/// [`list`](BlobProvider::list) asks the mount's listing endpoint
/// ([`LIST_PARAM`](Self::LIST_PARAM)), which answers the keys under the
/// requested prefix as a JSON array.
///
/// A write is an error: a process on a remote package store forks its edits into a
/// [`StoreFork`] rather than writing back.
///
/// Compiles wherever `json` does, so the mount's listing contract is one
/// declaration; a read needs a transport (wasm's fetch api, or `ureq`/`reqwest`
/// natively) and errors naming that without one.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_insert = BlobStore::on_insert::<Self>)]
pub struct HttpStore {
	/// The url the store is mounted at, absolute or origin-relative.
	base: Url,
	/// Optional subdirectory prefix for all keys.
	#[reflect(default)]
	subdir: Option<FwdPath>,
}

impl HttpStore {
	/// The query flag that turns a mount request into a listing of the keys
	/// under its path, `GET /package/docs?list` answering `["a.md", "b/c.md"]`
	/// relative to `docs`: the `list` param `ServeBlobs` declares, which a
	/// strict mount refuses under any other name.
	pub const LIST_PARAM: &'static str = "list";

	/// A store mounted at `base`.
	pub fn new(base: impl Into<Url>) -> Self {
		Self {
			base: base.into().with_rooted(true),
			subdir: None,
		}
	}

	/// The store an http [`StoreUri`] names: its origin (the page's own when
	/// absent) with the path prefix as the mount.
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		let StoreUri::Http {
			origin,
			path_prefix,
		} = uri
		else {
			bevybail!("`{uri}` is not an http store");
		};
		let base = origin
			.as_deref()
			.map(Url::coerce)
			.unwrap_or(Url::ROOT)
			.with_rooted(true);
		Self::new(match path_prefix {
			Some(prefix) => push_segments(base, prefix),
			None => base,
		})
		.xok()
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<FwdPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// The url `path` is fetched at: the base, the subdir, then the key.
	fn url(&self, path: &FwdPath) -> Url {
		let root = match &self.subdir {
			Some(subdir) => push_segments(self.base.clone(), subdir),
			None => self.base.clone(),
		};
		push_segments(root, path)
	}

	/// A read-only store: every write is this error.
	fn read_only(&self) -> BevyError {
		bevyhow!(
			"`http:{}` is served over http and read-only: fork it into a local \
			 store (`--store-fork`) to write",
			self.base
		)
	}
}

/// `url` extended by every segment of `path`, so a key with a `/` lands as
/// its own segments rather than one escaped one.
fn push_segments(url: Url, path: &FwdPath) -> Url {
	path.segments()
		.into_iter()
		.fold(url, |url, segment| url.push(segment))
}

/// Fetch `url`, a non-2xx status becoming its [`HttpError`].
async fn fetch(url: Url) -> Result<Response> {
	Request::get(url).send().await?.into_result().await?.xok()
}

impl StoreBackend for HttpStore {
	fn id(&self) -> &'static str { "http" }

	fn describe(&self) -> String { self.root_key().to_string() }

	fn region(&self) -> Option<String> { None }

	/// The mount answers its listing: an unreachable or unmounted one is a
	/// store that does not exist.
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let list = self.list();
		Box::pin(async move { list.await.is_ok().xok() })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		Box::pin(async move { Err(err) })
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		Box::pin(async move { Err(err) })
	}
}

impl BlobProvider for HttpStore {
	fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: FwdPath) -> Box<dyn BlobProvider> {
		Box::new(HttpStore {
			base: self.base.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn base(&self) -> Box<dyn BlobProvider> {
		Box::new(HttpStore {
			base: self.base.clone(),
			subdir: None,
		})
	}

	fn root_key(&self) -> SmolStr { format!("http:{}", self.base).into() }

	fn subdir(&self) -> FwdPath { self.subdir.clone().unwrap_or_default() }

	fn insert(&self, _path: &FwdPath, _body: Bytes) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		Box::pin(async move { Err(err) })
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
		let url = self.url(&FwdPath::default()).with_flag(Self::LIST_PARAM);
		Box::pin(async move { fetch(url).await?.json::<Vec<FwdPath>>().await })
	}

	fn get(&self, path: &FwdPath) -> SendBoxedFuture<Result<Bytes>> {
		let url = self.url(path);
		Box::pin(async move { fetch(url).await?.bytes().await })
	}

	/// A fetch whose status answers: a 404 is absence, any other failure is
	/// the store's.
	fn exists(&self, path: &FwdPath) -> SendBoxedFuture<Result<bool>> {
		let url = self.url(path);
		Box::pin(async move {
			let response = Request::head(url).send().await?;
			match response.status() {
				StatusCode::NOT_FOUND => Ok(false),
				status if status.is_ok() => Ok(true),
				_ => Err(response.into_error().await.into()),
			}
		})
	}

	fn remove(&self, _path: &FwdPath) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		Box::pin(async move { Err(err) })
	}

	/// The url a key is served at, absolute only: an origin-relative mount
	/// has no public url of its own, so a consumer streams the bytes instead.
	fn public_url(
		&self,
		path: &FwdPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let url = self.url(path);
		Box::pin(async move {
			url.authority().is_some().then(|| url.to_string()).xok()
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Keys land below the mount and the subdir, each segment its own.
	#[beet_core::test]
	fn builds_key_urls() {
		HttpStore::from_uri(
			&StoreUri::parse("https://beet.org/package").unwrap(),
		)
		.unwrap()
		.with_subdir(fwd_path!("docs"))
		.url(&fwd_path!("a/b.md"))
		.to_string()
		.xpect_eq("https://beet.org/package/docs/a/b.md");
		// an origin-relative mount is a rooted path, resolved by the transport
		HttpStore::from_uri(&StoreUri::parse("http:package").unwrap())
			.unwrap()
			.url(&fwd_path!("main.bsx"))
			.to_string()
			.xpect_eq("/package/main.bsx");
		HttpStore::from_uri(&StoreUri::parse("http").unwrap())
			.unwrap()
			.url(&fwd_path!("main.bsx"))
			.to_string()
			.xpect_eq("/main.bsx");
	}

	/// The store is read-only: a write names the store fork as the way to write.
	#[beet_core::test]
	async fn writes_are_refused() {
		HttpStore::new("https://beet.org/package")
			.insert(&fwd_path!("a.txt"), bytes::Bytes::from_static(b"hi"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("read-only");
	}
}
