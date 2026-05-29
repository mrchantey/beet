//! Storage abstraction for S3, filesystem, in-memory, and browser backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryStore`]: Ephemeral storage for testing
//! - [`FsStore`]: Local filesystem storage (native only)
//! - [`LocalStorageStore`]: Browser localStorage (WASM only)
//! - [`S3Store`]: AWS S3 storage (requires `aws_sdk` feature)
//! - [`DynamoStore`]: AWS DynamoDB storage (requires `aws_sdk` feature)
//!
//! Use [`StorePlugin`] to register store types for world serialization.
//! Concrete store types (like [`FsStore`], [`S3Store`]) are Components
//! that auto-insert a type-erased [`BlobStore`] via on_add hooks.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let store = BlobStore::temp();
//! store.insert(&SmolPath::from("hello.txt"), "world").await?;
//! let data = store.get(&SmolPath::from("hello.txt")).await?;
//! # Ok(())
//! # }
//! ```
// no_std core: the BlobStore facade, provider trait, in-memory store, and the
// reactive/handle wrappers. The concrete backends below are std-only.
mod blob;
mod blob_store_provider;
pub use blob_store_provider::*;
mod blob_store;
mod in_memory_store;
pub use blob::*;
pub use blob_store::*;
pub use in_memory_store::*;
// reactive store handle (signals/effects/spawn_local) — std-only
#[cfg(feature = "std")]
mod store_item;
#[cfg(feature = "std")]
pub use store_item::*;

#[cfg(all(feature = "json", feature = "std"))]
mod analytics;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
mod aws_cli;
#[cfg(feature = "world_serde")]
mod world_serde_store;
#[cfg(all(feature = "json", feature = "std"))]
mod table;
#[cfg(all(feature = "json", feature = "std"))]
pub use analytics::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
pub use aws_cli::*;
#[cfg(all(feature = "json", feature = "std"))]
pub use table::*;
#[cfg(feature = "world_serde")]
pub use world_serde_store::*;
#[cfg(feature = "std")]
mod fs_store;
#[cfg(target_arch = "wasm32")]
mod indexed_db_store;
#[cfg(target_arch = "wasm32")]
mod local_storage_store;
#[cfg(feature = "std")]
pub use fs_store::*;
#[cfg(target_arch = "wasm32")]
pub use indexed_db_store::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use s3_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod s3_store;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod s3_fs_store;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use s3_fs_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use dynamo_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod dynamo_store;

#[cfg(feature = "std")]
use beet_core::prelude::*;

/// Plugin that registers store types for world serialization.
#[cfg(feature = "std")]
#[derive(Default)]
pub struct StorePlugin;

#[cfg(feature = "std")]
impl Plugin for StorePlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<FsStore>()
			.register_type::<TypedBlob<FsStore>>();

		#[cfg(feature = "world_serde")]
		app.add_systems(PostUpdate, load_world_serde_on_insert);

		#[cfg(target_arch = "wasm32")]
		app.register_type::<LocalStorageStore>()
			.register_type::<TypedBlob<LocalStorageStore>>()
			.register_type::<IndexedDbStore>()
			.register_type::<TypedBlob<IndexedDbStore>>();

		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		app.register_type::<S3Store>()
			.register_type::<DynamoStore>()
			.register_type::<TypedBlob<S3Store>>()
			.register_type::<TypedBlob<DynamoStore>>();
	}
}
