use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// What a store knows about an object without handing over its bytes: its
/// size and, where the backend can answer one, its MD5 hex digest. MD5 because
/// that is the etag S3 reports for a single-part object, so a local file and
/// a stored object compare by the same number and a mirror
/// ([`BlobSync`](crate::prelude::BlobSync)) skips what already matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobStat {
	/// The object's size in bytes.
	pub size: u64,
	/// The MD5 hex digest, `None` where the backend cannot answer one (an S3
	/// multipart upload's etag is not an MD5).
	pub md5: Option<SmolStr>,
}

impl BlobStat {
	/// The stat of `bytes`, digested here.
	pub fn of(bytes: &[u8]) -> Self {
		use md5::Digest;
		let digest = md5::Md5::digest(bytes)
			.iter()
			.map(|byte| format!("{byte:02x}"))
			.collect::<String>();
		Self {
			size: bytes.len() as u64,
			md5: Some(digest.into()),
		}
	}

	/// Whether an object with this stat is the same content as `other`: equal
	/// sizes and equal digests. A side that cannot answer a digest never
	/// matches, so a mirror copies rather than guesses.
	pub fn matches(&self, other: &Self) -> bool {
		self.size == other.size
			&& matches!((&self.md5, &other.md5), (Some(a), Some(b)) if a == b)
	}
}

/// Trait for store storage backends (S3, filesystem, memory, etc.).
///
/// Implementations provide the actual storage operations for [`BlobStore`].
/// Each provider stores all required state internally (store name, region,
/// connection info, etc.) so that no external context is needed.
pub trait BlobStoreProvider: 'static + Send + Sync {
	/// Returns a boxed clone of this provider.
	fn box_clone(&self) -> Box<dyn BlobStoreProvider>;

	/// Returns a new provider scoped to the given subdirectory.
	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider>;

	/// The unscoped store at this store's root: the same backing with no subdir,
	/// the one [`root_key`](Self::root_key) names. A watcher keys its events to it
	/// so they route through [`did_change`](Self::did_change) whichever scope
	/// registered the watch.
	fn base(&self) -> Box<dyn BlobStoreProvider>;

	/// A view of this store rooted at `root` (a cleaned path relative to the
	/// store, which may climb above it with a leading `..`), plus `entry_name`
	/// re-expressed relative to that root. The seam behind
	/// [`BlobStore::rebase_repo`].
	///
	/// The default is the key-prefix view every provider supports
	/// ([`with_subdir`](Self::with_subdir)), erroring loudly when `root` walks
	/// above the store (`..`): a store with no parent universe cannot widen, so
	/// an entry declaring a root above it names a mis-published store — its own
	/// directory was synced instead of its declared universe. A provider whose
	/// backing *does* have a parent universe (an [`FsStore`]'s filesystem)
	/// overrides this to re-root above.
	fn rebase(
		&self,
		entry_name: &RelPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobStoreProvider>, RelPath)> {
		if root.first_segment() == Some("..") {
			bevybail!(
				"entry `{entry_name}` declares a `<RepoRoot>` above the store \
				itself (`{root}`), and a `{}` store has no parent universe. \
				This usually means a mis-published store: the entry's directory \
				was synced instead of its declared root.",
				self.id()
			);
		}
		let root = RelPath::new(root);
		let entry_name = entry_name.strip_prefix(&root).ok_or_else(|| {
			bevyhow!(
				"entry `{entry_name}` is not under its declared store root \
				`{root}`"
			)
		})?;
		let provider = match root.is_empty() {
			true => self.box_clone(),
			false => self.with_subdir(root),
		};
		(provider, entry_name).xok()
	}

	/// Create a type-erased [`Blob`] handle for a single object managed by
	/// this provider. Prefer the typed [`FsStore::blob`], [`S3Store::blob`]
	/// etc. when you need world serialization.
	fn erased_blob(&self, path: RelPath) -> Blob {
		Blob::new(BlobStore::new(self.box_clone()), path)
	}

	/// Stable family discriminator, ie `"fs"`, `"memory"`, `"localstorage"`,
	/// `"s3"`.
	fn id(&self) -> &'static str;

	/// [`id`](Self::id) plus the backing-instance identity, *without* the subdir.
	/// Two stores with the same `root_key` are the same effective store:
	/// - fs:           `"fs:{path}"`             (the base store directory)
	/// - memory:       `"memory:{name}"`         (the backing's name)
	/// - localstorage: `"localstorage:{store_name}"`
	/// - s3:           `"s3:{bucket}"`           (unwatchable)
	fn root_key(&self) -> SmolStr;

	/// This store's `subdir` within its [`root_key`](Self::root_key), empty for
	/// the root. Used for per-event routing.
	fn subdir(&self) -> RelPath { RelPath::default() }

	/// The precise directory a native watcher should observe for this store, ie an
	/// [`FsStore`]'s `effective_root` (base joined with subdir). `None` for a store
	/// with no watchable directory (memory, S3). Drives [`WatchDir`](crate::prelude::WatchDir),
	/// so only the mounted subtree is watched, never the whole store root.
	fn watch_dir(&self) -> Option<AbsPath> { None }

	/// The store's base path, ie the directory its [`root_key`](Self::root_key) keys
	/// to, used to strip a watched path back to a base-relative [`BlobEvent`] so it
	/// routes via [`did_change`](Self::did_change). `None` for a non-fs store.
	fn base_dir(&self) -> Option<AbsPath> { None }

	/// True if `event` concerns an object inside this store's scope: same
	/// backing, and the event's root-relative location is this scope or a child
	/// of it.
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.root_key() == event.store.root_key()
			&& key_covers(&self.subdir(), &event.root_relative_path())
	}

	/// Returns the provider's region, if applicable.
	fn region(&self) -> Option<String>;

	/// Where this store actually lives, ie `s3:beet-site--prod--app
	/// (us-west-2)`. The one thing an operator needs from a store that will not
	/// answer, so it belongs in any error naming one.
	fn describe(&self) -> String {
		match self.region() {
			Some(region) => format!("{} ({region})", self.root_key()),
			None => self.root_key().to_string(),
		}
	}

	/// Check if store exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let exists = store.store_exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>>;

	/// Create store (may take 10+ seconds for some services like DynamoDB).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store already exists.
	fn store_create(&self) -> SendBoxedFuture<Result>;

	/// Remove store (destructive operation!).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store doesn't exist.
	fn store_remove(&self) -> SendBoxedFuture<Result>;

	/// Ensure store exists, creating if needed.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_try_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn store_try_create(&self) -> SendBoxedFuture<Result> {
		let exists_fut = self.store_exists();
		let create_fut = self.store_create();
		Box::pin(async move {
			if exists_fut.await? {
				Ok(())
			} else {
				create_fut.await
			}
		})
	}

	/// Check if store is empty (contains no objects).
	fn store_is_empty(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.box_clone();
		Box::pin(async move { this.list().await?.is_empty().xok() })
	}

	/// Insert object into store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result>;

	/// List all objects in store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let paths = store.list().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>>;

	/// Get object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let data = store.get(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>>;

	/// Check if object exists in store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let exists = store.exists(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>>;

	/// Remove object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.remove(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result>;

	/// The object's [`BlobStat`], `None` when it does not exist. The default
	/// reads the object and digests it; a backend that answers from metadata
	/// (S3's `HeadObject`) overrides it.
	fn stat(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<BlobStat>>> {
		let this = self.box_clone();
		let path = path.clone();
		Box::pin(async move {
			if !this.exists(&path).await? {
				return Ok(None);
			}
			this.get(&path)
				.await
				.map(|bytes| Some(BlobStat::of(&bytes)))
		})
	}

	/// Every object with its [`BlobStat`], what a mirror diffs. The default
	/// lists then stats each object; S3 answers it from the listing alone.
	fn list_stats(&self) -> SendBoxedFuture<Result<Vec<(RelPath, BlobStat)>>> {
		let this = self.box_clone();
		Box::pin(async move {
			let paths = this.list().await?;
			async_ext::try_join_all_bounded(
				16,
				paths.into_iter().map(|path| {
					let this = this.box_clone();
					async move {
						let stat =
							this.stat(&path).await?.ok_or_else(|| {
								bevyhow!(
									"object {path} vanished between list and stat"
								)
							})?;
						Ok((path, stat))
					}
				}),
			)
			.await
		})
	}

	/// Get public URL of object.
	/// - fs: `file:///data/stores/my-store/key`
	/// - s3: `https://my-store.s3.us-west-2.amazonaws.com/key`
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let url = store.public_url(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

impl BlobStoreProvider for Box<dyn BlobStoreProvider> {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> {
		self.as_ref().box_clone()
	}
	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		self.as_ref().with_subdir(path)
	}
	fn base(&self) -> Box<dyn BlobStoreProvider> { self.as_ref().base() }
	fn rebase(
		&self,
		entry_name: &RelPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobStoreProvider>, RelPath)> {
		self.as_ref().rebase(entry_name, root)
	}
	fn id(&self) -> &'static str { self.as_ref().id() }
	fn root_key(&self) -> SmolStr { self.as_ref().root_key() }
	fn subdir(&self) -> RelPath { self.as_ref().subdir() }
	fn watch_dir(&self) -> Option<AbsPath> { self.as_ref().watch_dir() }
	fn base_dir(&self) -> Option<AbsPath> { self.as_ref().base_dir() }
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.as_ref().did_change(event)
	}
	fn region(&self) -> Option<String> { self.as_ref().region() }
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().store_exists()
	}
	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.as_ref().store_create()
	}
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.as_ref().store_remove()
	}
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.as_ref().insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		self.as_ref().list()
	}
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		self.as_ref().get(path)
	}
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().exists(path)
	}
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.as_ref().remove(path)
	}
	fn stat(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<BlobStat>>> {
		self.as_ref().stat(path)
	}
	fn list_stats(&self) -> SendBoxedFuture<Result<Vec<(RelPath, BlobStat)>>> {
		self.as_ref().list_stats()
	}
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.as_ref().public_url(path)
	}
}
