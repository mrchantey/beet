use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A bucket representing both local filesystem and remote S3 storage.
/// Uses [`ServiceAccess`] at runtime to determine which backing
/// store to delegate to.
#[derive(Debug, Clone, Get, SetWith, Component)]
#[component(on_add = Bucket::on_add::<Self>)]
pub struct S3FsBucket {
	/// Local filesystem bucket.
	#[set_with(skip)]
	fs_bucket: FsBucket,
	/// Remote S3 bucket.
	#[set_with(skip)]
	s3_bucket: S3Bucket,
	/// Runtime flag used to determine which bucket to use.
	service_access: ServiceAccess,
}

impl S3FsBucket {
	/// Create a new dual-mode bucket from filesystem and S3 components.
	pub fn new(fs_bucket: FsBucket, s3_bucket: S3Bucket) -> Self {
		Self {
			fs_bucket,
			s3_bucket,
			service_access: ServiceAccess::Local,
		}
	}

	/// Returns a reference to the active bucket provider based on
	/// the current [`ServiceAccess`] mode.
	fn active(&self) -> &dyn BucketProvider {
		match self.service_access {
			ServiceAccess::Local => &self.fs_bucket,
			ServiceAccess::Remote => &self.s3_bucket,
		}
	}
}

impl BucketProvider for S3FsBucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> {
		Box::new(self.clone())
	}
	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		self.active().with_subdir(path)
	}
	fn region(&self) -> Option<String> {
		self.active().region()
	}
	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.active().bucket_exists()
	}
	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		self.active().bucket_create()
	}
	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		self.active().bucket_remove()
	}
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.active().insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		self.active().list()
	}
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		self.active().get(path)
	}
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		self.active().exists(path)
	}
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.active().remove(path)
	}
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.active().public_url(path)
	}
}
