//! Object storage abstraction for S3, filesystem, and in-memory backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryBucket`]: Ephemeral storage for testing
//! - [`FsBucket`]: Local filesystem storage (native only)
//! - [`LocalStorageBucket`]: Browser localStorage (WASM only)
//! - [`S3Bucket`]: AWS S3 storage (requires `aws_sdk` feature)
//! - [`DynamoBucket`]: AWS DynamoDB storage (requires `aws_sdk` feature)
//!
//! Use [`BucketPlugin`] to register bucket types for scene serialization.
//! Concrete bucket types (like [`FsBucket`], [`S3Bucket`]) are Components
//! that auto-insert a type-erased [`Bucket`] via on_add hooks.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let bucket = Bucket::temp();
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
mod bucket_provider;
pub use bucket_provider::*;
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
mod in_memory_bucket;
pub use blob::*;
pub use bucket::*;
pub use bucket_item::*;
pub use in_memory_bucket::*;
#[cfg(feature = "bevy_scene")]
pub use scene_store::*;
mod fs_bucket;
#[cfg(target_arch = "wasm32")]
mod local_storage_bucket;
pub use fs_bucket::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_bucket::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use s3_bucket::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod s3_bucket;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use dynamo_bucket::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod dynamo_bucket;

use beet_core::prelude::*;

/// Plugin that registers bucket types for scene serialization.
///
/// Registers concrete bucket types for scene serialization.
#[derive(Default)]
pub struct BucketPlugin;

impl Plugin for BucketPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<FsBucket>()
			.register_type::<TypedBlob<FsBucket>>();

		#[cfg(feature = "bevy_scene")]
		app.add_systems(PostUpdate, load_scenes_on_insert);

		#[cfg(target_arch = "wasm32")]
		app.register_type::<LocalStorageBucket>()
			.register_type::<TypedBlob<LocalStorageBucket>>();

		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		app.register_type::<S3Bucket>()
			.register_type::<DynamoBucket>()
			.register_type::<TypedBlob<S3Bucket>>()
			.register_type::<TypedBlob<DynamoBucket>>();
	}
}
