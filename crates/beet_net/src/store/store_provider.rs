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
	/// `memory`.
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
}

/// Apply `func` to every variant's store, the one match the three accessors
/// share.
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
		}
	};
}

impl StoreProvider {
	/// The store `uri` names. A relative `fs:` path resolves against the cwd,
	/// the context dir a caller with a better one pins first
	/// ([`StoreUri::rooted_at`]).
	pub fn from_uri(uri: &StoreUri) -> Result<Self> {
		match uri {
			StoreUri::Fs { path } => path
				.as_deref()
				.unwrap_or(".")
				.xmap(AbsPathBuf::new)?
				.xmap(FsStore::new)
				.xmap(Self::Fs),
			StoreUri::Memory => Self::Memory(InMemoryStore::new()),
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
			StoreUri::LocalStorage { store, prefix } => {
				LocalStorageStore::new(store.clone())
					.xmap(|store| match prefix {
						Some(prefix) => store.with_subdir(prefix.as_str()),
						None => store,
					})
					.xmap(Self::LocalStorage)
			}
			#[cfg(target_arch = "wasm32")]
			StoreUri::IndexedDb { db, prefix } => IndexedDbStore::new(db.clone())
				.xmap(|store| match prefix {
					Some(prefix) => store.with_subdir(prefix.as_str()),
					None => store,
				})
				.xmap(Self::IndexedDb),
			#[cfg(not(target_arch = "wasm32"))]
			StoreUri::LocalStorage { .. } | StoreUri::IndexedDb { .. } => {
				bevybail!(
					"store `{uri}` is browser storage, only available on wasm"
				)
			}
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
		StoreProvider::from_uri(&StoreUri::Memory)
			.unwrap()
			.into_blob_store()
			.id()
			.xpect_eq("memory");
	}

	/// The concrete component lands, and its hook derives the erased store
	/// beside it.
	#[beet_core::test]
	fn inserting_derives_the_erased_store() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		StoreProvider::from_uri(&StoreUri::Memory)
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
}
