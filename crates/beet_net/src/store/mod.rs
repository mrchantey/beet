//! Storage abstraction for S3, filesystem, in-memory, and browser backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryStore`]: Ephemeral storage for testing
//! - [`FsStore`]: Local filesystem storage, cross-platform via `fs_ext` (the deno
//!   runner's fs globals back it on wasm)
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
mod blob_event;
mod blob_store_provider;
pub use blob_event::*;
pub use blob_store_provider::*;
mod blob_store;
mod in_memory_store;
mod store_path;
pub use blob::*;
pub use blob_store::*;
pub use in_memory_store::*;
pub use store_path::*;

// the analytics types + emission need only serde (which `std` pulls); the store
// persistence inside needs `json` (the TableStore surface) and gates itself.
#[cfg(feature = "std")]
mod analytics;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
mod aws_cli;
#[cfg(all(feature = "json", feature = "std"))]
mod table;
#[cfg(feature = "template_serde")]
mod template_store;
#[cfg(feature = "std")]
pub use analytics::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
pub use aws_cli::*;
#[cfg(all(feature = "json", feature = "std"))]
pub use table::*;
#[cfg(feature = "template_serde")]
pub use template_store::*;
// the `WatchDir` registration component (any `std` target) and the notify-based
// directory watcher backing it. The watcher itself is native+fs only for now (deno
// directory watching is unimplemented), so a wasm `FsStore` works without live
// reload; the `WatchDir` component is inert there. A future wasm watcher slots into
// this same seam (see `StorePlugin`).
#[cfg(feature = "std")]
mod fs_blob_watchers;
#[cfg(feature = "std")]
mod fs_store;
#[cfg(feature = "std")]
pub use fs_blob_watchers::*;
#[cfg(target_arch = "wasm32")]
mod indexed_db_store;
#[cfg(target_arch = "wasm32")]
mod local_storage_store;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod r2_workers_store;
#[cfg(feature = "std")]
pub use fs_store::*;
#[cfg(target_arch = "wasm32")]
pub use indexed_db_store::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_store::*;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub use r2_workers_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use s3_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod s3_fs_store;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
mod s3_store;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use dynamo_store::*;
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub use s3_fs_store::*;
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
			.register_type::<TypedBlob<FsStore>>()
			.register_type::<InMemoryStore>()
			.register_type::<TypedBlob<InMemoryStore>>()
			// store-path components: resolve a scoped store / blob from the nearest
			// ancestor store, kept correct as stores churn above them.
			.register_type::<DirPath>()
			.register_type::<BlobPath>()
			.add_observer(on_insert_dir_path)
			.add_observer(on_insert_blob_path)
			.add_observer(on_insert_store)
			.add_observer(on_remove_store);

		// reactive substrate: global event bus, drain, and the two propagation
		// observers marking matching BlobStore / Blob components Changed.
		app.init_resource::<BlobEventBus>()
			.add_systems(PreUpdate, drain_blob_events)
			.add_observer(propagate_blob_store_changes)
			.add_observer(propagate_blob_changes)
			.add_observer(add_memory_store_watcher)
			.add_observer(remove_memory_store_watcher);

		// fs watcher lifecycle, one notify debouncer per watched `WatchDir`. Native-only:
		// deno directory watching is unimplemented, so a wasm `FsStore` serves reads
		// through `fs_ext` with no live reload (the `WatchDir` component is inert there).
		// A future wasm watcher wires the same `add_watch_dir`/`remove_watch_dir` seam.
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		app.init_resource::<FsBlobWatchers>()
			.add_observer(add_watch_dir)
			.add_observer(remove_watch_dir);

		#[cfg(feature = "template_serde")]
		app.add_systems(PostUpdate, load_template_on_insert);

		// wasm localStorage watcher lifecycle (NonSend, owns the JS closure)
		#[cfg(target_arch = "wasm32")]
		app.register_type::<LocalStorageStore>()
			.register_type::<TypedBlob<LocalStorageStore>>()
			.register_type::<IndexedDbStore>()
			.register_type::<TypedBlob<IndexedDbStore>>()
			.init_non_send::<LocalStorageBlobWatcher>()
			.add_observer(add_local_storage_store_watcher)
			.add_observer(remove_local_storage_store_watcher);

		// the Cloudflare R2 binding store, registered so a deployed Worker can
		// resolve `<.. R2WorkersStore>` from markup and serialize it.
		#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
		app.register_type::<R2WorkersStore>()
			.register_type::<TypedBlob<R2WorkersStore>>();

		#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
		app.register_type::<S3Store>()
			.register_type::<DynamoStore>()
			.register_type::<TypedBlob<S3Store>>()
			.register_type::<TypedBlob<DynamoStore>>();
	}
}
