//! Blobs: bytes at paths, see [`BlobStore`]. A blob store holds bytes and a
//! table store rows, and neither implies the other: each is declared.
// no_std core: the BlobStore handle, provider trait, in-memory store, and the
// reactive/handle wrappers. The concrete backends below are std-only.
mod blob;
mod blob_event;
mod blob_provider;
mod blob_store;
// the store-agnostic mirror, over the provider trait's `list_stats`.
mod blob_sync;
mod in_memory_blob_store;
// a local store forked off an upstream one: pure composition, so it rides the
// core.
mod store_fork;
mod store_path;
pub use blob::*;
pub use blob_event::*;
pub use blob_provider::*;
pub use blob_store::*;
pub use blob_sync::*;
pub use in_memory_blob_store::*;
pub use store_fork::*;
pub use store_path::*;

// the http-served store: its listing endpoint answers json.
#[cfg(all(feature = "std", feature = "json"))]
mod http_store;
#[cfg(all(feature = "std", feature = "json"))]
pub use http_store::*;
// one sync between any two stores, by uri or by declaration.
#[cfg(feature = "std")]
mod store_sync;
#[cfg(feature = "std")]
pub use store_sync::*;
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
#[cfg(feature = "std")]
pub use fs_store::*;
// the browser stores ride the `std` store surface (`BlobEventBus`, the
// send_wrapper/futures bridge, and the base64 blob encoding).
#[cfg(all(target_arch = "wasm32", feature = "std"))]
mod indexed_db_store;
#[cfg(all(target_arch = "wasm32", feature = "std"))]
mod local_storage_store;
#[cfg(all(target_arch = "wasm32", feature = "std"))]
pub use indexed_db_store::*;
#[cfg(all(target_arch = "wasm32", feature = "std"))]
pub use local_storage_store::*;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod r2_workers_store;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub use r2_workers_store::*;
#[cfg(feature = "aws")]
mod s3_store;
#[cfg(feature = "aws")]
pub use s3_store::*;
#[cfg(feature = "sqlite")]
mod sqlite_blob_store;
#[cfg(feature = "sqlite")]
pub use sqlite_blob_store::*;
