//! [`HttpStore`]: a store read over http, the client half of a
//! `<ServeBlobs>` mount.

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A store published over http, read-only: a site's repo store as its
/// `<ServeBlobs prefix="repo"/>` serves it, read by the browser process the
/// site's page boots and by a native process pulling the site into a terminal
/// (`beet --repo=https://beet.org/repo --server=tui`).
///
/// Every key is fetched at `{base}/{key}` through [`Request::send`], so an
/// origin-relative base (`http:repo`) resolves against the page's origin in a
/// browser and the canonical loopback server natively. A miss is the mount's
/// 404, so a consumer distinguishes an absent file from a broken store.
/// [`list`](BlobStoreProvider::list) asks the mount's listing endpoint
/// ([`LIST_PARAM`](Self::LIST_PARAM)), which answers the keys under the
/// requested prefix as a JSON array.
///
/// A write is an error: a process on a remote repo forks its edits into an
/// [`OverlayStore`] rather than writing back.
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
	subdir: Option<RelPath>,
}

impl HttpStore {
	/// The query flag that turns a mount request into a listing of the keys
	/// under its path, `GET /repo/docs?list` answering `["a.md", "b/c.md"]`
	/// relative to `docs`. Declared here so the server and this client cannot
	/// disagree about its name.
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
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// The url `path` is fetched at: the base, the subdir, then the key.
	fn url(&self, path: &RelPath) -> Url {
		let root = match &self.subdir {
			Some(subdir) => push_segments(self.base.clone(), subdir),
			None => self.base.clone(),
		};
		push_segments(root, path)
	}

	/// A read-only store: every write is this error.
	fn read_only(&self) -> BevyError {
		bevyhow!(
			"`http:{}` is served over http and read-only: compose a local store \
			 over it (`--overlay`) to write",
			self.base
		)
	}
}

/// `url` extended by every segment of `path`, so a key with a `/` lands as
/// its own segments rather than one escaped one.
fn push_segments(url: Url, path: &RelPath) -> Url {
	path.segments()
		.into_iter()
		.fold(url, |url, segment| url.push(segment))
}

/// Fetch `url`, a non-2xx status becoming its [`HttpError`].
async fn fetch(url: Url) -> Result<Response> {
	send(Request::get(url)).await?.into_result().await?.xok()
}

cfg_if! {
	if #[cfg(target_arch = "wasm32")] {
		/// Box a fetch for the provider contract: the browser's fetch future is
		/// not `Send`, and the store is only ever polled on the one thread.
		fn boxed<T>(
			fut: impl 'static + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(send_wrapper::SendWrapper::new(fut))
		}

		/// The fetch api.
		async fn send(request: Request) -> Result<Response> {
			request.send().await
		}
	} else if #[cfg(any(feature = "ureq", feature = "reqwest"))] {
		/// Box a fetch for the provider contract.
		fn boxed<T>(
			fut: impl 'static + Send + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(fut)
		}

		/// The compiled native transport, whose future is `Send`.
		async fn send(request: Request) -> Result<Response> {
			request.send().await
		}
	} else {
		/// Box a fetch for the provider contract.
		fn boxed<T>(
			fut: impl 'static + Send + Future<Output = T>,
		) -> SendBoxedFuture<T> {
			Box::pin(fut)
		}

		/// No transport compiled in: the store exists (a scene declares it,
		/// a uri names it) but cannot be read from this build. Not routed
		/// through [`Request::send`], whose runtime-installed fallback is a
		/// no_std hook with no `Send` future to give a provider.
		async fn send(request: Request) -> Result<Response> {
			bevybail!(
				"cannot fetch `{}`: an http store needs a transport (enable \
				 `ureq` or `reqwest`)",
				request.url()
			)
		}
	}
}

impl BlobStoreProvider for HttpStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		Box::new(HttpStore {
			base: self.base.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn base(&self) -> Box<dyn BlobStoreProvider> {
		Box::new(HttpStore {
			base: self.base.clone(),
			subdir: None,
		})
	}

	fn id(&self) -> &'static str { "http" }

	fn root_key(&self) -> SmolStr { format!("http:{}", self.base).into() }

	fn subdir(&self) -> RelPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	/// The mount answers its listing: an unreachable or unmounted one is a
	/// store that does not exist.
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let list = self.list();
		boxed(async move { list.await.is_ok().xok() })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		boxed(async move { Err(err) })
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		boxed(async move { Err(err) })
	}

	fn insert(&self, _path: &RelPath, _body: Bytes) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		boxed(async move { Err(err) })
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let url = self.url(&RelPath::default()).with_flag(Self::LIST_PARAM);
		boxed(async move { fetch(url).await?.json::<Vec<RelPath>>().await })
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let url = self.url(path);
		boxed(async move { fetch(url).await?.bytes().await })
	}

	/// A fetch whose status answers: a 404 is absence, any other failure is
	/// the store's.
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let url = self.url(path);
		boxed(async move {
			let response = send(Request::head(url)).await?;
			match response.status() {
				StatusCode::NOT_FOUND => Ok(false),
				status if status.is_ok() => Ok(true),
				_ => Err(response.into_error().await.into()),
			}
		})
	}

	fn remove(&self, _path: &RelPath) -> SendBoxedFuture<Result> {
		let err = self.read_only();
		boxed(async move { Err(err) })
	}

	/// The url a key is served at, absolute only: an origin-relative mount
	/// has no public url of its own, so a consumer streams the bytes instead.
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let url = self.url(path);
		boxed(async move {
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
		HttpStore::from_uri(&StoreUri::parse("https://beet.org/repo").unwrap())
			.unwrap()
			.with_subdir("docs")
			.url(&RelPath::new("a/b.md"))
			.to_string()
			.xpect_eq("https://beet.org/repo/docs/a/b.md");
		// an origin-relative mount is a rooted path, resolved by the transport
		HttpStore::from_uri(&StoreUri::parse("http:repo").unwrap())
			.unwrap()
			.url(&RelPath::new("main.bsx"))
			.to_string()
			.xpect_eq("/repo/main.bsx");
		HttpStore::from_uri(&StoreUri::parse("http").unwrap())
			.unwrap()
			.url(&RelPath::new("main.bsx"))
			.to_string()
			.xpect_eq("/main.bsx");
	}

	/// The store is read-only: a write names the overlay as the way to write.
	#[beet_core::test]
	async fn writes_are_refused() {
		HttpStore::new("https://beet.org/repo")
			.insert(&RelPath::new("a.txt"), bytes::Bytes::from_static(b"hi"))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("read-only");
	}
}
