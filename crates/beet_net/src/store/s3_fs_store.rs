use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A store representing both local filesystem and remote S3 storage.
/// Uses [`ServiceAccess`] at runtime to determine which backing
/// store to delegate to.
#[derive(Debug, Clone, Get, SetWith, Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct S3FsStore {
	/// Local filesystem store.
	#[set_with(skip)]
	fs_store: FsStore,
	/// Remote S3 store.
	#[set_with(skip)]
	s3_store: S3Store,
	/// Runtime flag used to determine which store to use.
	service_access: ServiceAccess,
}

impl S3FsStore {
	/// Create a new dual-mode store from filesystem and S3 components.
	pub fn new(fs_store: FsStore, s3_store: S3Store) -> Self {
		Self {
			fs_store,
			s3_store,
			service_access: ServiceAccess::Local,
		}
	}

	/// Returns a reference to the active store provider based on
	/// the current [`ServiceAccess`] mode.
	fn active(&self) -> &dyn BlobStoreProvider {
		match self.service_access {
			ServiceAccess::Local => &self.fs_store,
			ServiceAccess::Remote => &self.s3_store,
		}
	}
}

impl BlobStoreProvider for S3FsStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }
	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		self.active().with_subdir(path)
	}
	fn id(&self) -> &'static str { self.active().id() }
	fn root_key(&self) -> SmolStr { self.active().root_key() }
	fn subdir(&self) -> SmolPath { self.active().subdir() }
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.active().did_change(event)
	}
	fn region(&self) -> Option<String> { self.active().region() }
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.active().store_exists()
	}
	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.active().store_create()
	}
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.active().store_remove()
	}
	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.active().insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		self.active().list()
	}
	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		self.active().get(path)
	}
	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		self.active().exists(path)
	}
	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		self.active().remove(path)
	}
	fn public_url(
		&self,
		path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.active().public_url(path)
	}
}
