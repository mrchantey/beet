use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use send_wrapper::SendWrapper;
use std::cell::RefCell;

/// A store provider backed by a Cloudflare R2 bucket through the Worker runtime's
/// R2 binding.
///
/// Unlike [`S3Store`], which reaches R2 over the S3-compatible HTTP API, this
/// reads through the in-isolate binding (`env.bucket(..)`), so no credentials or
/// network round-trip to the S3 endpoint are needed: the deployed Worker streams
/// the site straight from R2. The live [`worker::Bucket`] is resolved per call
/// from the ambient [`worker::Env`] stashed in [`set_worker_env`] at the top of
/// the `fetch` handler, mirroring how [`IndexedDbStore`] resolves the ambient
/// IndexedDB from `window`.
///
/// Every async method runs inside a [`SendWrapper`] since the underlying
/// `worker::Bucket` JS handles are `!Send`; the Worker runtime is single-threaded
/// so this never actually crosses a thread.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct R2WorkersStore {
	/// The R2 binding name, as declared in `wrangler.toml`.
	binding: SmolStr,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
}

thread_local! {
	/// The ambient Worker [`Env`](worker::Env), set at the top of the `fetch`
	/// handler so a [`R2WorkersStore`] can resolve its live bucket binding. The
	/// runtime is single-threaded, so a thread-local is the isolate-global slot.
	static WORKER_ENV: RefCell<Option<worker::Env>> = const { RefCell::new(None) };
}

/// Stash the request's [`worker::Env`] so any [`R2WorkersStore`] can resolve its
/// live bucket binding for the duration of the `fetch` invocation. Call this at
/// the top of the Worker entry, before building or driving the app.
pub fn set_worker_env(env: worker::Env) {
	WORKER_ENV.with(|slot| *slot.borrow_mut() = Some(env));
}

impl R2WorkersStore {
	/// Creates a new R2-binding-backed store provider for the given binding name.
	pub fn new(binding: impl Into<SmolStr>) -> Self {
		Self {
			binding: binding.into(),
			subdir: None,
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<SmolPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// Prefix `path` with this store's subdir, yielding the full object key.
	fn effective_key(&self, path: &SmolPath) -> String {
		match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		}
	}

	/// The R2 object version of `path` (its `head` metadata), or `None` if the
	/// object is absent. Used as a cheap rebuild marker: a re-synced object gets a
	/// new version, so a deployed Worker can rebuild its world on the next request.
	pub async fn head_version(
		&self,
		path: &SmolPath,
	) -> Result<Option<String>> {
		let bucket = self.bucket();
		let key = self.effective_key(path);
		SendWrapper::new(async move {
			bucket?
				.head(key)
				.await?
				.map(|object| object.version())
				.xok()
		})
		.await
	}

	/// Resolve the live [`worker::Bucket`] from the ambient [`worker::Env`].
	fn bucket(&self) -> Result<worker::Bucket> {
		WORKER_ENV.with(|slot| {
			slot.borrow()
				.as_ref()
				.ok_or_else(|| {
					bevyhow!(
						"R2WorkersStore: no worker::Env set; call set_worker_env \
						at the top of the fetch handler"
					)
				})?
				.bucket(&self.binding)
				.map_err(|err| bevyhow!("R2 binding `{}`: {err}", self.binding))
		})
	}
}

impl BlobStoreProvider for R2WorkersStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(R2WorkersStore {
			binding: self.binding.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn id(&self) -> &'static str { "r2_workers" }

	fn root_key(&self) -> SmolStr {
		format!("r2_workers:{}", self.binding).into()
	}

	fn subdir(&self) -> SmolPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		// a binding either resolves or it does not; resolving is the existence check.
		let exists = self.bucket().is_ok();
		Box::pin(SendWrapper::new(async move { exists.xok() }))
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		// R2 buckets are provisioned out-of-band (wrangler / dashboard); the binding
		// is the bucket, so creation is a no-op here.
		Box::pin(SendWrapper::new(async move { ().xok() }))
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		Box::pin(SendWrapper::new(async move {
			bevybail!("R2WorkersStore does not support removing the bucket")
		}))
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let bucket = self.bucket();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			bucket?.put(key, body.to_vec()).execute().await?;
			().xok()
		}))
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let bucket = self.bucket();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			bucket?.head(key).await?.is_some().xok()
		}))
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let bucket = self.bucket();
		let prefix = self.subdir.as_ref().map(|sub| format!("{sub}/"));
		Box::pin(SendWrapper::new(async move {
			let bucket = bucket?;
			let mut paths = Vec::new();
			let mut cursor = None;
			// page through every object, stripping the subdir prefix off each key.
			loop {
				let mut req = bucket.list();
				if let Some(prefix) = &prefix {
					req = req.prefix(prefix.clone());
				}
				if let Some(cursor) = &cursor {
					req = req.cursor(cursor);
				}
				let objects = req.execute().await?;
				paths.extend(objects.objects().into_iter().filter_map(
					|object| {
						let key = object.key();
						let rel = match &prefix {
							Some(prefix) => {
								key.strip_prefix(prefix.as_str())?
							}
							None => &key,
						};
						Some(SmolPath::new(rel))
					},
				));
				match objects.truncated() {
					true => cursor = objects.cursor(),
					false => break,
				}
				if cursor.is_none() {
					break;
				}
			}
			paths.xok()
		}))
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let bucket = self.bucket();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			let object = bucket?
				.get(&key)
				.execute()
				.await?
				.ok_or_else(|| bevyhow!("Object not found: {key}"))?;
			let body = object
				.body()
				.ok_or_else(|| bevyhow!("Object has no body: {key}"))?;
			Bytes::from(body.bytes().await?).xok()
		}))
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let bucket = self.bucket();
		let key = self.effective_key(path);
		Box::pin(SendWrapper::new(async move {
			bucket?.delete(key).await?;
			().xok()
		}))
	}

	fn public_url(
		&self,
		_path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		// no public URL: the Worker streams the object through the binding rather
		// than handing out a bucket URL.
		Box::pin(SendWrapper::new(async move { None.xok() }))
	}
}
