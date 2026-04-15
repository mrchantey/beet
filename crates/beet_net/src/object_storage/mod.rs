//! Object storage abstraction for S3, filesystem, and in-memory backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryBucket`]: Ephemeral storage for testing
//! - [`FsBucket`]: Local filesystem storage (native only)
//! - [`LocalStorageBucket`]: Browser localStorage (WASM only)
//! - [`S3Bucket`]: AWS S3 storage (requires `aws` feature)
//! - [`DynamoBucket`]: AWS DynamoDB storage (requires `aws` feature)
//!
//! Use [`BucketPlugin`] to register bucket types for scene serialization.
//! [`TypedBucket`] and [`TypedBlob`] are the serializable wrappers that
//! auto-insert their erased counterparts ([`Bucket`] and [`Blob`]) on add.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let bucket = temp_bucket();
//! bucket.insert(&RelPath::from("hello.txt"), "world").await?;
//! let data = bucket.get(&RelPath::from("hello.txt")).await?;
//! # Ok(())
//! # }
//! ```
#[cfg(feature = "json")]
mod analytics;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
mod aws_cli;
mod blob;
mod bucket;
#[cfg(feature = "bevy_scene")]
mod scene_store;
#[cfg(feature = "json")]
mod table;
#[cfg(feature = "json")]
pub use analytics::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
pub use aws_cli::*;
#[cfg(feature = "json")]
pub use table::*;
mod bucket_item;
mod in_memory_provider;
pub use blob::*;
pub use bucket::*;
pub use bucket_item::*;
pub use in_memory_provider::*;
#[cfg(feature = "bevy_scene")]
pub use scene_store::*;
mod fs_provider;
#[cfg(target_arch = "wasm32")]
mod local_storage_provider;
pub use fs_provider::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
pub use s3_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
mod s3_provider;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
pub use dynamo_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
mod dynamo_provider;

use beet_core::prelude::*;

/// Plugin that registers bucket types for scene serialization.
///
/// Registers [`TypedBucket`] and [`TypedBlob`] instantiations for each
/// provider so that scenes containing bucket references can be serialized
/// and deserialized.
#[derive(Default)]
pub struct BucketPlugin;

impl Plugin for BucketPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<FsBucket>()
			.register_type::<TypedBucket<FsBucket>>()
			.register_type::<TypedBlob<FsBucket>>()
			.add_systems(PostUpdate, load_scenes_on_insert);

		#[cfg(target_arch = "wasm32")]
		app.register_type::<LocalStorageBucket>()
			.register_type::<TypedBucket<LocalStorageBucket>>()
			.register_type::<TypedBlob<LocalStorageBucket>>();

		#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
		app.register_type::<S3Bucket>()
			.register_type::<DynamoBucket>()
			.register_type::<TypedBucket<S3Bucket>>()
			.register_type::<TypedBlob<S3Bucket>>()
			.register_type::<TypedBucket<DynamoBucket>>()
			.register_type::<TypedBlob<DynamoBucket>>();
	}
}
