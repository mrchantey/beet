//! The one seam between a [`StoreUri`] and the concrete store component it
//! names.

use crate::prelude::*;
use beet_core::prelude::*;

/// The concrete store component a [`StoreUri`] names, built by
/// [`from_uri`](Self::from_uri): the single construction seam for store
/// selection, shared by the binary's entry resolution, the `check`/`serve`/
/// `export-static` commands and every declaration's runtime attach, so store
/// selection is uri-driven everywhere and no call site matches on a kind.
///
/// A concrete component rather than an erased [`BlobStore`], because the
/// component's own insert hook is what lands the erased currencies (the
/// [`BlobStore`], and the `TableStore` under `json`) on its entity, and a
/// concrete component reflects, so a scene serializes and inspects the store
/// it was given. [`insert`](Self::insert) lands it on an entity;
/// [`into_blob_store`](Self::into_blob_store) erases it for a world-free
/// caller.
///
/// A kind whose backend this build did not compile errors with guidance rather
/// than degrading: the store concept is target-agnostic, only the backend is
/// gated.
#[derive(Debug, Clone)]
pub enum StoreProvider {
	/// `fs` / `fs:<path>`.
	Fs(FsStore),
	/// `memory://<name>`.
	Memory(InMemoryStore),
	/// `s3://<bucket>`.
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	S3(S3Store),
	/// `dynamo://<table>`.
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	Dynamo(DynamoStore),
	/// `local-storage://<store>`.
	#[cfg(target_arch = "wasm32")]
	LocalStorage(LocalStorageStore),
	/// `indexed-db://<db>`.
	#[cfg(target_arch = "wasm32")]
	IndexedDb(IndexedDbStore),
	/// `r2://<binding>`.
	#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
	R2(R2WorkersStore),
	/// `http://<host>/<prefix>` / `http:<prefix>`.
	#[cfg(feature = "json")]
	Http(HttpStore),
	/// `sqlite:<path>`.
	#[cfg(all(feature = "sqlite", not(target_arch = "wasm32")))]
	Sqlite(SqliteStore),
}

/// Apply `$body` to every variant's store, the one match `insert` and
/// `into_blob_store` share.
macro_rules! each_store {
	($this:expr, |$store:ident| $body:expr) => {
		match $this {
			StoreProvider::Fs($store) => $body,
			StoreProvider::Memory($store) => $body,
			#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
			StoreProvider::S3($store) => $body,
			#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
			StoreProvider::Dynamo($store) => $body,
			#[cfg(target_arch = "wasm32")]
			StoreProvider::LocalStorage($store) => $body,
			#[cfg(target_arch = "wasm32")]
			StoreProvider::IndexedDb($store) => $body,
			#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
			StoreProvider::R2($store) => $body,
			#[cfg(feature = "json")]
			StoreProvider::Http($store) => $body,
			#[cfg(all(feature = "sqlite", not(target_arch = "wasm32")))]
			StoreProvider::Sqlite($store) => $body,
		}
	};
}

impl StoreProvider {
	/// The store `uri` names. A relative `fs:` path resolves against the cwd,
	/// the context dir a caller with a better one pins first
	/// ([`StoreUri::rooted_at`]).
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		match uri {
			StoreUri::Fs { path_prefix } => path_prefix
				.as_ref()
				.map(SmolPath::as_str)
				.unwrap_or(".")
				.xmap(AbsPath::new)?
				.xmap(FsStore::new)
				.xmap(Self::Fs),
			StoreUri::Memory { name, path_prefix } => {
				InMemoryStore::named(name.clone())
					.xmap(|store| match path_prefix {
						Some(prefix) => store.with_subdir(prefix.clone()),
						None => store,
					})
					.xmap(Self::Memory)
			}
			#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
			StoreUri::S3 { .. } => Self::S3(S3Store::from_uri(uri)?),
			#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
			StoreUri::Dynamo { .. } => Self::Dynamo(DynamoStore::from_uri(uri)?),
			#[cfg(not(all(feature = "aws_sdk", not(target_arch = "wasm32"))))]
			StoreUri::S3 { .. } | StoreUri::Dynamo { .. } => bevybail!(
				"store `{uri}` requires a compiled AWS backend (enable the \
				 `aws_sdk` feature, native only)"
			),
			#[cfg(target_arch = "wasm32")]
			StoreUri::LocalStorage { name, path_prefix } => {
				LocalStorageStore::new(name.clone())
					.xmap(|store| match path_prefix {
						Some(prefix) => store.with_subdir(prefix.clone()),
						None => store,
					})
					.xmap(Self::LocalStorage)
			}
			#[cfg(target_arch = "wasm32")]
			StoreUri::IndexedDb { name, path_prefix } => {
				IndexedDbStore::new(name.clone())
					.xmap(|store| match path_prefix {
						Some(prefix) => store.with_subdir(prefix.clone()),
						None => store,
					})
					.xmap(Self::IndexedDb)
			}
			#[cfg(not(target_arch = "wasm32"))]
			StoreUri::LocalStorage { .. } | StoreUri::IndexedDb { .. } => {
				bevybail!(
					"store `{uri}` is browser storage, only available on wasm"
				)
			}
			#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
			StoreUri::R2 { .. } => Self::R2(R2WorkersStore::from_uri(uri)?),
			#[cfg(not(all(target_arch = "wasm32", feature = "cloudflare")))]
			StoreUri::R2 { .. } => bevybail!(
				"store `{uri}` is an R2 binding, reachable only from a \
				 Cloudflare Worker (a wasm build with the `cloudflare` feature)"
			),
			#[cfg(feature = "json")]
			StoreUri::Http { .. } => Self::Http(HttpStore::from_uri(uri)?),
			#[cfg(not(feature = "json"))]
			StoreUri::Http { .. } => bevybail!(
				"store `{uri}` is served over http, whose listing is json \
				 (enable the `json` feature)"
			),
			#[cfg(all(feature = "sqlite", not(target_arch = "wasm32")))]
			StoreUri::Sqlite { .. } => Self::Sqlite(SqliteStore::from_uri(uri)?),
			#[cfg(not(all(feature = "sqlite", not(target_arch = "wasm32"))))]
			StoreUri::Sqlite { .. } => bevybail!(
				"store `{uri}` is a SQLite database (enable the `sqlite` \
				 feature, native only)"
			),
		}
		.xok()
	}

	/// The store `uri` names forked into `store_fork` ([`StoreFork`]), or the
	/// store alone. The composition behind `--repo` + `--store-fork`, erased
	/// since the pair has no single component. The fork is marked for the
	/// repo it forks ([`StoreUri::fork_mark`]), so a browser's first edit
	/// says so to the next served page.
	pub fn compose(
		uri: &StoreUri,
		store_fork: Option<&StoreUri>,
	) -> Result<BlobStore> {
		let upstream = Self::from_uri(uri)?.into_blob_store();
		match store_fork {
			Some(store_fork) => Self::from_uri(store_fork)?
				.into_blob_store()
				.xmap(|local| {
					StoreFork::new(local, upstream).with_mark(uri.fork_mark())
				})
				.xmap(BlobStore::new),
			None => upstream,
		}
		.xok()
	}

	/// Land the store on `entity`, its insert hook deriving the erased
	/// currencies at the next flush.
	pub fn insert(self, entity: &mut EntityWorldMut) {
		each_store!(self, |store| {
			entity.insert(store);
		})
	}

	/// The erased store, for a caller with no entity to land it on.
	pub fn into_blob_store(self) -> BlobStore {
		each_store!(self, |store| BlobStore::new(store))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A uri builds the backend it names, rooted where it says.
	#[beet_core::test]
	fn builds_the_named_backend() {
		StoreProvider::from_uri(&StoreUri::parse("fs:/data").unwrap())
			.unwrap()
			.into_blob_store()
			.base_dir()
			.unwrap()
			.to_string()
			.xpect_eq("/data");
		StoreProvider::from_uri(&StoreUri::parse("memory://m/docs").unwrap())
			.unwrap()
			.into_blob_store()
			.xmap(|store| (store.root_key(), store.subdir()))
			.xpect_eq(("memory:m".into(), RelPath::new("docs")));
	}

	/// A database file uri builds the sqlite backend, its relative path
	/// resolved against the cwd and its blobs unscoped.
	#[cfg(all(feature = "sqlite", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn builds_a_sqlite_backend() {
		let store = StoreProvider::from_uri(
			&StoreUri::parse("sqlite:data.db").unwrap(),
		)
		.unwrap()
		.into_blob_store();
		store.id().xpect_eq("sqlite");
		store
			.root_key()
			.xpect_eq(format!("sqlite:{}", AbsPath::new("data.db").unwrap()));
		store.subdir().xpect_eq(RelPath::default());
	}

	/// A memory uri names one backing: every build of it reads the same data,
	/// so a store a test seeds by name is the store the uri names.
	#[beet_core::test]
	async fn a_memory_uri_names_one_backing() {
		let uri = StoreUri::parse("memory://provider-shared").unwrap();
		let build = || StoreProvider::from_uri(&uri).unwrap().into_blob_store();
		let seeded = build();
		seeded.insert(&RelPath::new("a.txt"), "hi").await.unwrap();
		build()
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from_static(b"hi"));
	}

	/// The concrete component lands, and its hook derives the erased store
	/// beside it.
	#[beet_core::test]
	fn inserting_derives_the_erased_store() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		StoreProvider::from_uri(&StoreUri::parse("memory://m").unwrap())
			.unwrap()
			.insert(&mut world.entity_mut(entity));
		world.flush();
		world.get::<InMemoryStore>(entity).xpect_some();
		world.get::<BlobStore>(entity).xpect_some();
	}

	/// A kind this build has no backend for errors with guidance.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn a_missing_backend_errors_with_guidance() {
		StoreProvider::from_uri(&StoreUri::parse("indexed-db://d").unwrap())
			.unwrap_err()
			.to_string()
			.xpect_contains("only available on wasm");
	}

	/// A store fork composes over the repo: the pair reads through and writes
	/// local, by name so a test reaches each half.
	#[beet_core::test]
	async fn composes_a_store_fork() {
		let upstream = StoreUri::parse("memory://compose-upstream").unwrap();
		let local = StoreUri::parse("memory://compose-local").unwrap();
		// a memory backing lives as long as a handle does
		let seeded = BlobStore::from_uri(&upstream).unwrap();
		seeded.insert(&RelPath::new("a.txt"), "up").await.unwrap();
		let store = StoreProvider::compose(&upstream, Some(&local)).unwrap();
		store.id().xpect_eq("fork");
		store
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_eq(bytes::Bytes::from_static(b"up"));
		store.insert(&RelPath::new("b.txt"), "local").await.unwrap();
		BlobStore::from_uri(&local)
			.unwrap()
			.exists(&RelPath::new("b.txt"))
			.await
			.unwrap()
			.xpect_true();
		StoreProvider::compose(&upstream, None)
			.unwrap()
			.id()
			.xpect_eq("memory");
	}
}
