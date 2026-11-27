use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bytes::Bytes;

/// Cross-service storage store for S3, filesystem, memory, or other providers.
///
/// Wraps an [`Arc<dyn BlobProvider>`] and delegates all operations to the
/// inner provider. Implements [`BlobProvider`] itself, so it can be used
/// anywhere a provider is expected. Bytes alone: a blob store is never a table
/// store, see `JsonTableStore` for rows over one.
#[derive(Clone, Component)]
pub struct BlobStore {
	/// Provider that handles store operations (S3, filesystem, memory, etc.)
	provider: Arc<dyn BlobProvider>,
}

impl core::fmt::Debug for BlobStore {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BlobStore").finish_non_exhaustive()
	}
}

impl BlobStore {
	/// How many reads a whole-store fan-out keeps in flight at once.
	///
	/// The single ceiling for every `get_all`-shaped read, blob or table. A
	/// remote store answers a listing with thousands of keys, and issuing that
	/// many concurrent requests exhausts the connection pool: every request past
	/// the pool's capacity fails its connect timeout, which a lossy read then
	/// reports as an unreadable row.
	pub const GET_ALL_CONCURRENCY: usize = 32;

	/// Creates a new store wrapping the given provider.
	pub fn new(provider: impl BlobProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// Creates a new store from a pre-existing [`Arc<dyn BlobProvider>`].
	pub fn from_arc(provider: Arc<dyn BlobProvider>) -> Self {
		Self { provider }
	}

	/// A new, empty [`InMemoryBlobStore`], for a test or scratch work.
	pub fn temp() -> BlobStore { BlobStore::new(InMemoryBlobStore::new()) }

	/// The erased store a [`StoreUri`] names, for a caller with no entity to
	/// land the concrete component on (entry resolution, a prune). See
	/// [`BlobStoreProvider::from_uri`], the one seam this rides.
	#[cfg(feature = "std")]
	pub fn from_uri(uri: &StoreUri) -> Result<BlobStore> {
		BlobStoreProvider::from_uri(uri)?.into_blob_store().xok()
	}

	/// The store and file name a path or uri ending in a file names: the
	/// store is the file's directory (or the uri without its last segment),
	/// the file its last segment. `~/a/b.toml` is `fs:~/a` + `b.toml`,
	/// `s3://bucket/dir/b.toml?region=x` is `s3://bucket/dir?region=x` +
	/// `b.toml`, and a bare `b.toml` is the cwd.
	#[cfg(feature = "std")]
	pub fn from_file_uri(path: &str) -> Result<(BlobStore, SmolStr)> {
		let path = path.trim();
		let (base, query) = path
			.split_once('?')
			.map(|(base, query)| (base, format!("?{query}")))
			.unwrap_or((path, String::new()));
		let is_uri = base.contains("://") || base.starts_with("fs:");
		let (dir, file) = match base.rsplit_once('/') {
			// `fs:` alone is the cwd
			Some((dir, file))
				if !dir.ends_with(':') && !dir.ends_with(":/") =>
			{
				(dir.to_string(), file)
			}
			Some(_) => bevybail!(
				"`{path}` names no file: a file uri is a store followed by the \
				file within it, ie `s3://bucket/secrets/x.toml`"
			),
			None => match is_uri {
				true => ("fs".to_string(), base.trim_start_matches("fs:")),
				false => (".".to_string(), base),
			},
		};
		if file.is_empty() {
			bevybail!("`{path}` names no file");
		}
		let uri = match is_uri {
			true => StoreUri::parse(&format!("{dir}{query}"))?,
			false => StoreUri::parse(&format!("fs:{dir}"))?,
		};
		(Self::from_uri(&uri)?, SmolStr::new(file)).xok()
	}

	/// Returns a new store scoped to the given subdirectory.
	pub fn with_subdir(&self, path: FwdPath) -> BlobStore {
		BlobStore::from_arc(Arc::from(self.provider.with_subdir(path)))
	}

	/// The unscoped store at this store's root (see
	/// [`BlobProvider::base`]): the same backing, no subdir.
	pub fn base(&self) -> BlobStore {
		BlobStore::from_arc(Arc::from(self.provider.base()))
	}

	/// Rebase this store and `entry_name` through the entry's declared
	/// `<PackageRoot src>`: `src` names a position relative to the entry
	/// document's own location *in this store*, so the new root is
	/// `normalize(parent(entry_name)/src)` and the entry name becomes the
	/// entry path relative to it.
	///
	/// Kind behavior rides [`BlobProvider::rebase`]: a filesystem store
	/// re-roots at the absolute resolved directory (fs has a parent universe,
	/// walking above the store is the point), every other store takes a
	/// key-prefix view of itself for a root at or under its own and errors
	/// loudly when the root escapes the store. That error is the feature: the
	/// declaration is a checkable invariant about store layout, and an entry
	/// whose declared universe is missing was mis-published (its directory was
	/// synced instead of its declared root).
	pub fn rebase_package(
		&self,
		entry_name: &str,
		src: &str,
	) -> Result<(BlobStore, String)> {
		let entry_name = FwdPath::new(entry_name)?;
		// the root is a plain path: `..` may climb above the store
		let root = entry_name
			.as_smol_path()
			.parent()
			.unwrap_or_default()
			.join(src);
		let (provider, entry_name) =
			self.provider.rebase(&entry_name, &root)?;
		(
			BlobStore::from_arc(Arc::from(provider)),
			entry_name.to_string(),
		)
			.xok()
	}

	/// Whether `other` resolves the same scope: same backing store and subdir. The
	/// [`DirPath`] change-detection uses it to skip a no-op re-scope, so a cascade of
	/// ancestor [`BlobStore`] inserts settles instead of re-firing forever.
	pub fn same_scope(&self, other: &BlobStore) -> bool {
		self.root_key() == other.root_key() && self.subdir() == other.subdir()
	}

	/// Get an object and infer the [`MediaType`] from its path extension.
	pub async fn get_media(&self, path: &FwdPath) -> Result<MediaBytes> {
		let media_type = path.media_type().unwrap_or(MediaType::Bytes);
		MediaBytes::new(media_type, self.get(path).await?).xok()
	}

	/// Create a [`Blob`] handle for a single object in this store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let store = BlobStore::temp();
	/// let blob = store.blob(fwd_path!("my-file.txt"));
	/// ```
	pub fn blob(&self, path: FwdPath) -> Blob { Blob::new(self.clone(), path) }

	/// Component hook that reads a concrete store component from
	/// the entity and inserts a [`BlobStore`] wrapping it.
	/// Use with `#[component(on_insert = BlobStore::on_insert::<MyStore>)]`.
	pub fn on_insert<T: Component + Clone + BlobProvider>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(provider) => {
				world
					.commands()
					.entity(cx.entity)
					.insert(BlobStore::new(provider));
			}
			Err(err) => {
				world.fallback_error_handler()(err, ErrorContext::Command {
					name: core::any::type_name_of_val(
						&BlobStore::on_insert::<T>,
					)
					.into(),
				});
			}
		}
	}

	/// Insert object into store.
	///
	/// Ergonomic wrapper accepting any `impl Into<Bytes>` (eg. `&str`, `Vec<u8>`).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.insert(&fwd_path!("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(
		&self,
		path: &FwdPath,
		body: impl Into<Bytes>,
	) -> Result {
		self.provider.insert(path, body.into()).await
	}

	/// Insert the contents of a local file without holding it in memory, see
	/// [`BlobProvider::insert_file`].
	#[cfg(feature = "std")]
	pub async fn insert_file(&self, path: &FwdPath, file: &AbsPath) -> Result {
		self.provider.insert_file(path, file).await
	}

	/// Insert object, failing if it already exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.try_insert(&fwd_path!("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn try_insert(
		&self,
		path: &FwdPath,
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
	/// let store = BlobStore::temp();
	/// let all_data = store.get_all().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Caution
	/// Expensive operation - prefer [`BlobProvider::list`] + [`BlobProvider::get`]
	pub async fn get_all(&self) -> Result<Vec<(FwdPath, Bytes)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let data = self.get(&path).await?;
				Ok::<_, BevyError>((path, data))
			})
			.xmap(|futures| {
				async_ext::try_join_all_bounded(
					Self::GET_ALL_CONCURRENCY,
					futures,
				)
			})
			.await
	}
}

impl StoreBackend for BlobStore {
	fn id(&self) -> &'static str { self.provider.id() }

	fn describe(&self) -> String { self.provider.describe() }

	fn region(&self) -> Option<String> { self.provider.region() }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.provider.store_exists()
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.provider.store_create()
	}

	fn store_try_create(&self) -> SendBoxedFuture<Result> {
		self.provider.store_try_create()
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.provider.store_remove()
	}
}

impl BlobProvider for BlobStore {
	fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }
	fn with_subdir(&self, path: FwdPath) -> Box<dyn BlobProvider> {
		self.provider.with_subdir(path)
	}
	fn base(&self) -> Box<dyn BlobProvider> { self.provider.base() }
	fn rebase(
		&self,
		entry_name: &FwdPath,
		root: &SmolPath,
	) -> Result<(Box<dyn BlobProvider>, FwdPath)> {
		self.provider.rebase(entry_name, root)
	}
	fn root_key(&self) -> SmolStr { self.provider.root_key() }
	fn subdir(&self) -> FwdPath { self.provider.subdir() }
	fn watch_dir(&self) -> Option<AbsPath> { self.provider.watch_dir() }
	fn base_dir(&self) -> Option<AbsPath> { self.provider.base_dir() }
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.provider.did_change(event)
	}
	fn matches_object(&self, event: &BlobEvent, path: &FwdPath) -> bool {
		self.provider.matches_object(event, path)
	}
	fn insert(&self, path: &FwdPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.provider.insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
		self.provider.list()
	}
	fn list_dir(&self, path: &FwdPath) -> SendBoxedFuture<Result<BlobDir>> {
		self.provider.list_dir(path)
	}
	fn get(&self, path: &FwdPath) -> SendBoxedFuture<Result<Bytes>> {
		self.provider.get(path)
	}
	fn exists(&self, path: &FwdPath) -> SendBoxedFuture<Result<bool>> {
		self.provider.exists(path)
	}
	fn remove(&self, path: &FwdPath) -> SendBoxedFuture<Result> {
		self.provider.remove(path)
	}
	fn conditional_writes(&self) -> bool { self.provider.conditional_writes() }
	fn get_versioned(
		&self,
		path: &FwdPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		self.provider.get_versioned(path)
	}
	fn insert_if(
		&self,
		path: &FwdPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		self.provider.insert_if(path, body, condition)
	}
	fn remove_if(
		&self,
		path: &FwdPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		self.provider.remove_if(path, condition)
	}
	fn stat(
		&self,
		path: &FwdPath,
	) -> SendBoxedFuture<Result<Option<BlobStat>>> {
		self.provider.stat(path)
	}
	fn size(&self, path: &FwdPath) -> SendBoxedFuture<Result<Option<u64>>> {
		self.provider.size(path)
	}
	fn list_stats(&self) -> SendBoxedFuture<Result<Vec<(FwdPath, BlobStat)>>> {
		self.provider.list_stats()
	}
	fn empty_links(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
		self.provider.empty_links()
	}
	fn public_url(
		&self,
		path: &FwdPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.provider.public_url(path)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A store with no parent universe seeded with a nested entry plus content
	/// beside and above it.
	async fn nested_store() -> BlobStore {
		let store = BlobStore::temp();
		store
			.insert(&fwd_path!("a/b/main.bsx"), "<Router/>")
			.await
			.unwrap();
		store
			.insert(&fwd_path!("a/shared.txt"), "shared")
			.await
			.unwrap();
		store
	}

	/// A nested entry declaring a root at an ancestor key takes a key-prefix
	/// view of the same store, and the view serves both the entry and content
	/// beside it.
	#[beet_core::test]
	async fn rebases_to_a_prefix_view() {
		let store = nested_store().await;
		let (rebased, entry_name) =
			store.rebase_package("a/b/main.bsx", "..").unwrap();
		entry_name.xpect_eq("b/main.bsx");
		rebased
			.get_media(&fwd_path!("b/main.bsx"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("<Router/>");
		rebased
			.get_media(&fwd_path!("shared.txt"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("shared");
	}

	/// A root resolving exactly to the store's own root is the store itself,
	/// the entry name unchanged: a nested entry declaring `../..` in a bucket
	/// published from its universe just works.
	#[beet_core::test]
	async fn root_level_rebase_is_identity() {
		let store = nested_store().await;
		let (rebased, entry_name) =
			store.rebase_package("a/b/main.bsx", "../..").unwrap();
		entry_name.xpect_eq("a/b/main.bsx");
		rebased.same_scope(&store).xpect_true();
	}

	/// A root escaping a store with no parent universe fails loudly naming the
	/// mis-publish, rather than asking the store for keys it cannot hold.
	#[beet_core::test]
	async fn escaping_root_names_the_mis_publish() {
		BlobStore::temp()
			.rebase_package("main.bsx", "../..")
			.unwrap_err()
			.to_string()
			.xpect_contains("mis-published");
	}

	/// A declared root the entry does not sit under is an authoring error.
	#[beet_core::test]
	async fn entry_outside_root_errors() {
		BlobStore::temp()
			.rebase_package("a/main.bsx", "sub")
			.unwrap_err()
			.to_string()
			.xpect_contains("not under its declared store root");
	}

	#[beet_core::test]
	fn splits_a_file_uri() {
		let (store, file) =
			BlobStore::from_file_uri("memory://files/dir/x.toml").unwrap();
		file.as_str().xpect_eq("x.toml");
		store.subdir().as_str().xpect_eq("dir");
		let (store, file) =
			BlobStore::from_file_uri("/tmp/beet/personal.toml").unwrap();
		file.as_str().xpect_eq("personal.toml");
		store.base_dir().unwrap().to_string().xpect_eq("/tmp/beet");
		BlobStore::from_file_uri("x.json")
			.unwrap()
			.0
			.id()
			.xpect_eq("fs");
		BlobStore::from_file_uri("memory://").unwrap_err();
		BlobStore::from_file_uri("dir/").unwrap_err();
	}
}

/// Test utilities for store providers.
#[cfg(test)]
pub mod store_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A conditional write checked and written as one step: absent on an
	/// existing object, matches on a changed one, and removes alike.
	async fn conditional(store: &BlobStore) {
		let path = fwd_path!("conditional/object");
		let is_conflict = |err: BevyError| {
			err.downcast_ref::<BlobConflict>().is_some().xpect_true();
		};
		store.get_versioned(&path).await.unwrap().xpect_none();
		let first = store
			.insert_if(&path, "one".into(), BlobCondition::Absent)
			.await
			.unwrap();
		let (bytes, version) =
			store.get_versioned(&path).await.unwrap().unwrap();
		bytes.xpect_eq(bytes::Bytes::from("one"));
		version.clone().xpect_eq(first.clone());
		store
			.insert_if(&path, "two".into(), BlobCondition::Absent)
			.await
			.unwrap_err()
			.xmap(is_conflict);
		let second = store
			.insert_if(
				&path,
				"two".into(),
				BlobCondition::Matches(first.clone()),
			)
			.await
			.unwrap();
		store
			.insert_if(
				&path,
				"three".into(),
				BlobCondition::Matches(first.clone()),
			)
			.await
			.unwrap_err()
			.xmap(is_conflict);
		store
			.get(&path)
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from("two"));
		store
			.remove_if(&path, BlobCondition::Matches(first.clone()))
			.await
			.unwrap_err()
			.xmap(is_conflict);
		store
			.remove_if(&path, BlobCondition::Absent)
			.await
			.unwrap_err()
			.xmap(is_conflict);
		store
			.remove_if(&path, BlobCondition::Matches(second))
			.await
			.unwrap();
		store.remove_if(&path, BlobCondition::Any).await.unwrap();
		store.remove_if(&path, BlobCondition::Absent).await.unwrap();
		store
			.remove_if(&path, BlobCondition::Matches(first))
			.await
			.unwrap_err()
			.xmap(is_conflict);
	}

	/// Runs the standard store provider test suite.
	pub async fn run(provider: impl BlobProvider) {
		let store = BlobStore::new(provider);
		store.store_remove().await.ok();
		store.store_exists().await.unwrap().xpect_false();
		store.store_try_create().await.unwrap();
		run_objects(&store).await;
		store.store_remove().await.unwrap();
		store.store_exists().await.unwrap().xpect_false();
	}

	/// Runs every case but the store's lifecycle, for a store provisioned
	/// rather than made by its test (a deployed bucket or table), leaving it
	/// as empty as it found it.
	pub async fn run_objects(store: &BlobStore) {
		let store = store.clone();
		let path = fwd_path!("test_path");
		let body = bytes::Bytes::from("test_body");
		store.exists(&path).await.unwrap().xpect_false();
		// a miss names what was missed, so whoever reports it says which
		store
			.get(&path)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("test_path");
		store.remove(&path).await.xpect_err();
		store.insert(&path, body.clone()).await.unwrap();
		store.store_exists().await.unwrap().xpect_true();
		store.exists(&path).await.unwrap().xpect_true();
		store.list().await.unwrap().xpect_eq(vec![path.clone()]);
		store.get(&path).await.unwrap().xpect_eq(body.clone());
		store.get(&path).await.unwrap().xpect_eq(body.clone());
		// the stat matches a local digest of the same bytes, whichever backend
		let stat = BlobStat::of(&body);
		store
			.stat(&path)
			.await
			.unwrap()
			.xpect_eq(Some(stat.clone()));
		store.size(&path).await.unwrap().xpect_eq(Some(9));
		store
			.size(&fwd_path!("missing"))
			.await
			.unwrap()
			.xpect_none();
		store
			.list_stats()
			.await
			.unwrap()
			.xpect_eq(vec![(path.clone(), stat)]);

		// one directory's children by name, a missing directory empty
		let leaf = fwd_path!("dir/nested/leaf.txt");
		let file = fwd_path!("dir/file.txt");
		store.insert(&leaf, body.clone()).await.unwrap();
		store.insert(&file, body.clone()).await.unwrap();
		store
			.list_dir(&FwdPath::default())
			.await
			.unwrap()
			.xpect_eq(BlobDir {
				dirs: vec!["dir".into()],
				files: vec!["test_path".into()],
			});
		store
			.list_dir(&fwd_path!("dir"))
			.await
			.unwrap()
			.xpect_eq(BlobDir {
				dirs: vec!["nested".into()],
				files: vec!["file.txt".into()],
			});
		store
			.list_dir(&fwd_path!("missing"))
			.await
			.unwrap()
			.xpect_eq(BlobDir::default());
		// a subdir lists its own keys, never a sibling sharing its name's start
		let sibling = fwd_path!("dir.txt");
		store.insert(&sibling, body.clone()).await.unwrap();
		let mut listed = store
			.with_subdir(fwd_path!("dir"))
			.list()
			.await
			.unwrap();
		listed.sort();
		listed.xpect_eq(vec![
			fwd_path!("file.txt"),
			fwd_path!("nested/leaf.txt"),
		]);
		store.remove(&sibling).await.unwrap();
		store.remove(&leaf).await.unwrap();
		store.remove(&file).await.unwrap();

		store.remove(&path).await.unwrap();
		store.get(&path).await.xpect_err();
		store.stat(&path).await.unwrap().xpect_none();
		conditional(&store).await;

		// a file inserts as its bytes and is left in place
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			let dir = TempDir::new_ws().unwrap();
			let file = dir.join("source.bin");
			fs_ext::write(&file, "from a file").unwrap();
			store.insert_file(&path, &file).await.unwrap();
			store
				.get(&path)
				.await
				.unwrap()
				.xpect_eq(bytes::Bytes::from("from a file"));
			fs_ext::exists(&file).unwrap().xpect_true();
			store.remove(&path).await.unwrap();
		}
	}
}
