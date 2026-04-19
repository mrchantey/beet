use crate::prelude::*;
use beet_core::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use bytes::Bytes;
use std::sync::Arc;

/// Cross-service storage bucket for S3, filesystem, memory, or other providers.
///
/// Wraps an [`Arc<dyn BucketProvider>`] and delegates all operations to the
/// inner provider. Implements [`BucketProvider`] itself, so it can be used
/// anywhere a provider is expected.
#[derive(Clone, Component)]
pub struct Bucket {
	/// Provider that handles bucket operations (S3, filesystem, memory, etc.)
	provider: Arc<dyn BucketProvider>,
}

impl std::fmt::Debug for Bucket {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Bucket").finish_non_exhaustive()
	}
}

impl Bucket {
	/// Creates a new bucket wrapping the given provider.
	pub fn new(provider: impl BucketProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// Creates a new bucket from a pre-existing [`Arc<dyn BucketProvider>`].
	pub fn from_arc(provider: Arc<dyn BucketProvider>) -> Self {
		Self { provider }
	}

	/// Create temporary in-memory bucket for testing.
	/// The returned bucket is pre-created and ready for immediate use.
	pub fn temp() -> Bucket { Bucket::new(InMemoryBucket::new()) }

	/// Create local bucket with platform-specific provider.
	/// - wasm: [`LocalStorageBucket`]
	/// - native: [`FsBucket`] at `.cache/buckets/<name>`
	pub fn new_local(name: impl Into<String>) -> Bucket {
		let name = name.into();
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				Bucket::new(LocalStorageBucket::new(name))
			} else {
				Bucket::new(FsBucket::new(
					AbsPathBuf::new_workspace_rel(format!(".cache/buckets/{name}"))
						.unwrap(),
				))
			}
		}
	}

	/// Select filesystem or S3 bucket based on [`ServiceAccess`] and feature flags.
	#[allow(unused_variables)]
	pub async fn new_s3_fs_selector(
		fs_path: AbsPathBuf,
		bucket_name: impl AsRef<str>,
		region: &str,
		access: ServiceAccess,
	) -> Bucket {
		let bucket_name = bucket_name.as_ref();
		match access {
			ServiceAccess::Local => {
				debug!("Bucket Selector - FS: {fs_path}");
				Bucket::new(FsBucket::new(fs_path))
			}
			ServiceAccess::Remote => {
				cfg_if! {
					if #[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))] {
						debug!("Bucket Selector - S3: {bucket_name}");
						let provider = S3Bucket::new(bucket_name, region);
						Bucket::new(provider)
					} else {
						debug!("Bucket Selector - FS (no aws_sdk or wasm): {fs_path}");
						Bucket::new(FsBucket::new(fs_path))
					}
				}
			}
		}
	}

	/// Returns a new bucket scoped to the given subdirectory.
	pub fn with_subdir(&self, path: RelPath) -> Bucket {
		Bucket::from_arc(Arc::from(self.provider.with_subdir(path)))
	}

	/// Create a [`BucketItem`] for working with a specific path.
	///
	/// # Example
	/// ```no_run
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let bucket = Bucket::temp();
	/// let item = bucket.item(RelPath::from("my-file.txt"));
	/// ```
	pub fn item(&self, path: RelPath) -> BucketItem {
		BucketItem::new(self.clone(), path)
	}

	/// Get an object and infer the [`MediaType`] from its path extension.
	pub async fn get_media(&self, path: &RelPath) -> Result<MediaBytes> {
		let media_type = MediaType::from_path(path.as_path());
		let bytes = self.get(path).await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Create a [`Blob`] handle for a single object in this bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let bucket = Bucket::temp();
	/// let blob = bucket.blob(RelPath::new("my-file.txt"));
	/// ```
	pub fn blob(&self, path: RelPath) -> Blob { Blob::new(self.clone(), path) }

	/// Component hook that reads a concrete bucket component from
	/// the entity and inserts a [`Bucket`] wrapping it.
	/// Use with `#[component(on_add = Bucket::on_add::<MyBucket>)]`.
	pub fn on_add<T: Component + Clone + BucketProvider>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(provider) => {
				world
					.commands()
					.entity(cx.entity)
					.insert(Bucket::new(provider));
			}
			Err(err) => {
				world.default_error_handler()(err, ErrorContext::Command {
					name: std::any::type_name_of_val(&Bucket::on_add::<T>)
						.into(),
				});
			}
		}
	}

	/// Insert object into bucket.
	///
	/// Ergonomic wrapper accepting any `impl Into<Bytes>` (eg. `&str`, `Vec<u8>`).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = Bucket::temp();
	/// bucket.insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(
		&self,
		path: &RelPath,
		body: impl Into<Bytes>,
	) -> Result {
		self.provider.insert(path, body.into()).await
	}

	/// Insert object, failing if it already exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = Bucket::temp();
	/// bucket.try_insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn try_insert(
		&self,
		path: &RelPath,
		body: impl Into<Bytes>,
	) -> Result {
		if self.exists(path).await? {
			bevybail!("Object already exists: {}", path)
		} else {
			self.insert(path, body).await
		}
	}

	/// Get all objects and their data.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = Bucket::temp();
	/// let all_data = bucket.get_all().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Caution
	/// Expensive operation - prefer [`BucketProvider::list`] + [`BucketProvider::get`]
	pub async fn get_all(&self) -> Result<Vec<(RelPath, Bytes)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let data = self.get(&path).await?;
				Ok::<_, BevyError>((path, data))
			})
			.xmap(async_ext::try_join_all)
			.await
	}
}

impl BucketProvider for Bucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }
	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		self.provider.with_subdir(path)
	}
	fn region(&self) -> Option<String> { self.provider.region() }
	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.provider.bucket_exists()
	}
	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		self.provider.bucket_create()
	}
	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		self.provider.bucket_remove()
	}
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.provider.insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		self.provider.list()
	}
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		self.provider.get(path)
	}
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		self.provider.exists(path)
	}
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.provider.remove(path)
	}
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.provider.public_url(path)
	}
}

/// Test utilities for bucket providers.
#[cfg(test)]
pub mod bucket_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Runs the standard bucket provider test suite.
	pub async fn run(provider: impl BucketProvider) {
		let bucket = Bucket::new(provider);
		let path = RelPath::from("test_path");
		let body = bytes::Bytes::from("test_body");
		bucket.bucket_remove().await.ok();
		bucket.bucket_exists().await.unwrap().xpect_false();
		bucket.bucket_try_create().await.unwrap();
		bucket.exists(&path).await.unwrap().xpect_false();
		bucket.remove(&path).await.xpect_err();
		bucket.insert(&path, body.clone()).await.unwrap();
		bucket.bucket_exists().await.unwrap().xpect_true();
		bucket.exists(&path).await.unwrap().xpect_true();
		bucket.list().await.unwrap().xpect_eq(vec![path.clone()]);
		bucket.get(&path).await.unwrap().xpect_eq(body.clone());
		bucket.get(&path).await.unwrap().xpect_eq(body);

		bucket.remove(&path).await.unwrap();
		bucket.get(&path).await.xpect_err();

		bucket.bucket_remove().await.unwrap();
		bucket.bucket_exists().await.unwrap().xpect_false();
	}
}
