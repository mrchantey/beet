//! The one seam between a [`StoreUri`] and the concrete store component it
//! names, as the kind it is asked for: [`BlobStoreProvider`] a store of bytes,
//! [`TableStoreProvider`] a store of rows.

use crate::prelude::*;
use beet_core::prelude::*;

/// The concrete blob store component a [`StoreUri`] names, built by
/// [`from_uri`](Self::from_uri): the single construction seam for store
/// selection, shared by the binary's entry resolution, the `check`/`serve`/
/// `export-static` commands and every declaration's runtime attach, so store
/// selection is uri-driven everywhere and no call site matches on a kind.
///
/// A concrete component rather than an erased [`BlobStore`], because the
/// component's own insert hook is what lands the [`BlobStore`] on its entity,
/// and a concrete component reflects, so a scene serializes and inspects the
/// store it was given. [`insert`](Self::insert) lands it on an entity;
/// [`into_blob_store`](Self::into_blob_store) erases it for a world-free
/// caller.
///
/// A kind whose backend this build did not compile errors with guidance rather
/// than degrading: the store concept is target-agnostic, only the backend is
/// gated. A uri naming a table store alone (`dynamo:`) is refused, see
/// [`TableStoreProvider`].
#[derive(Debug, Clone)]
pub enum BlobStoreProvider {
	/// `fs` / `fs:<path>`.
	Fs(FsStore),
	/// `memory://<name>`.
	Memory(InMemoryBlobStore),
	/// `s3://<bucket>`.
	#[cfg(feature = "aws")]
	S3(S3Store),
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
	#[cfg(feature = "sqlite")]
	Sqlite(SqliteBlobStore),
}

/// Apply `$body` to every variant's store, the one match `insert` and
/// `into_blob_store` share.
macro_rules! each_store {
	($this:expr, |$store:ident| $body:expr) => {
		match $this {
			BlobStoreProvider::Fs($store) => $body,
			BlobStoreProvider::Memory($store) => $body,
			#[cfg(feature = "aws")]
			BlobStoreProvider::S3($store) => $body,
			#[cfg(target_arch = "wasm32")]
			BlobStoreProvider::LocalStorage($store) => $body,
			#[cfg(target_arch = "wasm32")]
			BlobStoreProvider::IndexedDb($store) => $body,
			#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
			BlobStoreProvider::R2($store) => $body,
			#[cfg(feature = "json")]
			BlobStoreProvider::Http($store) => $body,
			#[cfg(feature = "sqlite")]
			BlobStoreProvider::Sqlite($store) => $body,
		}
	};
}

impl BlobStoreProvider {
	/// The blob store `uri` names. A relative `fs:` path resolves against the cwd,
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
				InMemoryBlobStore::named(name.clone())
					.xmap(|store| match path_prefix {
						Some(prefix) => store.with_subdir(prefix.clone()),
						None => store,
					})
					.xmap(Self::Memory)
			}
			#[cfg(feature = "aws")]
			StoreUri::S3 { .. } => Self::S3(S3Store::from_uri(uri)?),
			StoreUri::Dynamo { .. } => bevybail!(
				"store `{uri}` is a table store and holds no blobs: name a \
				 bucket or a directory for bytes, ie `s3://<bucket>`"
			),
			#[cfg(not(feature = "aws"))]
			StoreUri::S3 { .. } => bevybail!(
				"store `{uri}` requires a compiled AWS backend (enable the \
				 `aws` feature)"
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
			#[cfg(feature = "sqlite")]
			StoreUri::Sqlite { .. } => Self::Sqlite(SqliteBlobStore::from_uri(uri)?),
			#[cfg(not(feature = "sqlite"))]
			StoreUri::Sqlite { .. } => bevybail!(
				"store `{uri}` is a SQLite database (enable the `sqlite` \
				 feature)"
			),
		}
		.xok()
	}

	/// The store `uri` names forked into `store_fork` ([`StoreFork`]), or the
	/// store alone. The composition behind `--package` + `--store-fork`, erased
	/// since the pair has no single component. The fork is marked for the
	/// package store it forks ([`StoreUri::fork_mark`]), so a browser's first edit
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
	/// [`BlobStore`] at the next flush.
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

/// The table store a [`StoreUri`] names, built by
/// [`from_uri`](Self::from_uri): [`BlobStoreProvider`]'s twin for rows. A uri
/// names a table store or a blob store by which is asked for, so a `sqlite:`
/// file answers both, and any blob uri asked for as a table store is a
/// [`JsonTableStore`] over it, since a caller asking for that names it by
/// asking.
#[cfg(feature = "json")]
#[derive(Debug, Clone)]
pub enum TableStoreProvider {
	/// `dynamo://<table>`.
	#[cfg(feature = "aws")]
	Dynamo(DynamoStore),
	/// `sqlite:<path>`.
	#[cfg(feature = "sqlite")]
	Sqlite(SqliteStore),
	/// A blob uri, its rows json files.
	Json(JsonTableProvider),
}

#[cfg(feature = "json")]
impl TableStoreProvider {
	/// The table store `uri` names, a relative path resolved as
	/// [`BlobStoreProvider::from_uri`] resolves one.
	///
	/// # Errors
	/// Errors for a kind this build has no backend for, and for a blob store
	/// no rows can be kept in (one read over http).
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		match uri {
			#[cfg(feature = "aws")]
			StoreUri::Dynamo { .. } => Self::Dynamo(DynamoStore::from_uri(uri)?),
			#[cfg(not(feature = "aws"))]
			StoreUri::Dynamo { .. } => bevybail!(
				"store `{uri}` requires a compiled AWS backend (enable the \
				 `aws` feature)"
			),
			#[cfg(feature = "sqlite")]
			StoreUri::Sqlite { .. } => Self::Sqlite(SqliteStore::from_uri(uri)?),
			uri => BlobStoreProvider::from_uri(uri)?
				.into_blob_store()
				.xmap(JsonTableProvider::new)?
				.xmap(Self::Json),
		}
		.xok()
	}

	/// Land the table store on `entity`, the concrete component where there
	/// is one. A blob uri lands its rows alone, never the blob store they are
	/// in.
	pub fn insert(self, entity: &mut EntityWorldMut) {
		match self {
			#[cfg(feature = "aws")]
			Self::Dynamo(store) => {
				entity.insert(store);
			}
			#[cfg(feature = "sqlite")]
			Self::Sqlite(store) => {
				entity.insert(store);
			}
			Self::Json(store) => TableStore::new(store).land(entity),
		}
	}

	/// The erased store, for a caller with no entity to land it on.
	pub fn into_table_store(self) -> TableStore {
		match self {
			#[cfg(feature = "aws")]
			Self::Dynamo(store) => TableStore::new(store),
			#[cfg(feature = "sqlite")]
			Self::Sqlite(store) => TableStore::new(store),
			Self::Json(store) => TableStore::new(store),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A uri builds the backend it names, rooted where it says.
	#[beet_core::test]
	fn builds_the_named_backend() {
		BlobStoreProvider::from_uri(&StoreUri::parse("fs:/data").unwrap())
			.unwrap()
			.into_blob_store()
			.base_dir()
			.unwrap()
			.to_string()
			.xpect_eq("/data");
		BlobStoreProvider::from_uri(
			&StoreUri::parse("memory://m/docs").unwrap(),
		)
		.unwrap()
		.into_blob_store()
		.xmap(|store| (store.root_key(), store.subdir()))
		.xpect_eq(("memory:m".into(), RelPath::new("docs")));
	}

	/// A database file uri builds the sqlite blob store, its relative path
	/// resolved against the cwd and its blobs unscoped.
	#[cfg(feature = "sqlite")]
	#[beet_core::test]
	fn builds_a_sqlite_backend() {
		let store = BlobStoreProvider::from_uri(
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

	/// A uri is the kind it is asked for: a database file either, a Dynamo
	/// table rows alone, and a blob uri rows as json files in it.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn a_uri_is_the_kind_asked_for() {
		let uri = |uri: &str| StoreUri::parse(uri).unwrap();
		TableStore::from_uri(&uri("memory://kinds"))
			.unwrap()
			.describe()
			.xpect_eq("json:memory:kinds/@root.json");
		BlobStore::from_uri(&uri("dynamo://t"))
			.unwrap_err()
			.to_string()
			.xpect_contains("holds no blobs")
			.xpect_contains("s3://<bucket>");
		TableStore::from_uri(&uri("http://localhost/package"))
			.unwrap_err()
			.to_string()
			.xpect_contains("conditional writes")
			.xpect_contains("SqliteStore");
		#[cfg(feature = "aws")]
		TableStore::from_uri(&uri("dynamo://t"))
			.unwrap()
			.describe()
			.xpect_eq("dynamo:t");
		#[cfg(feature = "sqlite")]
		{
			TableStore::from_uri(&uri("sqlite:/data.db"))
				.unwrap()
				.supports(Capability::Sql)
				.xpect_true();
			BlobStore::from_uri(&uri("sqlite:/data.db"))
				.unwrap()
				.id()
				.xpect_eq("sqlite");
		}
	}

	/// Landing a uri as either kind lands that kind alone.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn lands_one_kind() {
		let mut world = World::new();
		let uri = StoreUri::parse("memory://lands-one-kind").unwrap();
		let blobs = world.spawn_empty().id();
		BlobStoreProvider::from_uri(&uri)
			.unwrap()
			.insert(&mut world.entity_mut(blobs));
		let tables = world.spawn_empty().id();
		TableStoreProvider::from_uri(&uri)
			.unwrap()
			.insert(&mut world.entity_mut(tables));
		world.flush();
		world.entity(blobs).contains::<BlobStore>().xpect_true();
		world.entity(blobs).contains::<TableStore>().xpect_false();
		world.entity(tables).contains::<BlobStore>().xpect_false();
		world.entity(tables).contains::<TableStore>().xpect_true();
	}

	/// A memory uri names one backing: every build of it reads the same data,
	/// so a store a test seeds by name is the store the uri names.
	#[beet_core::test]
	async fn a_memory_uri_names_one_backing() {
		let uri = StoreUri::parse("memory://provider-shared").unwrap();
		let build =
			|| BlobStoreProvider::from_uri(&uri).unwrap().into_blob_store();
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
		BlobStoreProvider::from_uri(&StoreUri::parse("memory://m").unwrap())
			.unwrap()
			.insert(&mut world.entity_mut(entity));
		world.flush();
		world.get::<InMemoryBlobStore>(entity).xpect_some();
		world.get::<BlobStore>(entity).xpect_some();
	}

	/// A kind this build has no backend for errors with guidance.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn a_missing_backend_errors_with_guidance() {
		BlobStoreProvider::from_uri(
			&StoreUri::parse("indexed-db://d").unwrap(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("only available on wasm");
	}

	/// A store fork composes over the package store: the pair reads through and writes
	/// local, by name so a test reaches each half.
	#[beet_core::test]
	async fn composes_a_store_fork() {
		let upstream = StoreUri::parse("memory://compose-upstream").unwrap();
		let local = StoreUri::parse("memory://compose-local").unwrap();
		// a memory backing lives as long as a handle does
		let seeded = BlobStore::from_uri(&upstream).unwrap();
		seeded.insert(&RelPath::new("a.txt"), "up").await.unwrap();
		let store =
			BlobStoreProvider::compose(&upstream, Some(&local)).unwrap();
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
		BlobStoreProvider::compose(&upstream, None)
			.unwrap()
			.id()
			.xpect_eq("memory");
	}
}
