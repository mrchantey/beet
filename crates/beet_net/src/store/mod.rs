//! Storage abstraction for S3, filesystem, in-memory, and browser backends.
//!
//! This module provides a unified API for storing and retrieving binary data
//! across different storage backends:
//!
//! - [`InMemoryBlobStore`]: blobs in process memory, for a test or a runtime
//!   whose bytes need not outlive it
//! - [`FsStore`]: Local filesystem storage, cross-platform via `fs_ext` (the deno
//!   runner's fs globals back it on wasm)
//! - [`LocalStorageStore`]: Browser localStorage (WASM only)
//! - [`S3Store`]: AWS S3 storage, and any S3-compatible service ie R2
//!   (requires `aws`)
//! - [`DynamoStore`]: tables in AWS DynamoDB (requires `aws`)
//! - [`SqliteStore`]: tables in a SQLite database file, with an SQL escape
//!   hatch (requires `sqlite`)
//! - [`SqliteBlobStore`]: blobs in a SQLite database file (requires `sqlite`)
//! - [`JsonTableStore`]: tables as json files in any blob store, for trying
//!   things in a small binary and for a browser tab (requires `json`)
//! - [`InMemoryTableStore`]: tables in process memory, for a test or a
//!   runtime whose rows need not outlive it (requires `json`)
//! - [`DurableObjectStore`]: tables whose every partition is a Cloudflare
//!   Durable Object, natively a [`PartitionHost`] of SQLite files (requires
//!   `json`)
//! - [`HttpStore`]: a store served over http (a `<ServeBlobs>` mount), read-only
//!   (requires `json`; reads need a transport, wasm's fetch or `ureq`/`reqwest`)
//! - [`StoreFork`]: a local store forked off an upstream one, the fork a
//!   process keeps of a repo it reads but does not own
//!
//! ## The repo store
//!
//! One store in an app is special: the [`RepoStore`], the canonical store an
//! entry loads through, holding the entry document, its templates, routes and
//! assets. A world carries exactly one (the marker errors on a second), and a
//! consumer reaches it either by ancestry (`AncestorQuery<&BlobStore>`, which
//! honours any intervening scope) or directly through [`RepoStore::get`]. Every
//! other store is a plain one: declared for a purpose and named by a
//! [`StoreRef`], or scoped out of an ancestor by a [`DirPath`].
//!
//! Use [`StorePlugin`] to register store types for world serialization.
//! Concrete store types (like [`FsStore`], [`S3Store`]) are Components whose
//! insert hooks derive the type-erased currency of what they hold: a
//! [`BlobStore`] for bytes or a `TableStore` for rows, never one from the
//! other, so a consumer resolves the kind it reads without naming a backend
//! and never reaches the wrong store. Every store is selected by a `StoreUri`
//! and built through `store_provider`, the one place a uri becomes a backend,
//! as the kind that is asked for.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let store = BlobStore::temp();
//! store.insert(&RelPath::from("hello.txt"), "world").await?;
//! let data = store.get(&RelPath::from("hello.txt")).await?;
//! # Ok(())
//! # }
//! ```
// blobs: the `BlobStore` handle, its provider trait, the in-memory store and
// every blob backend.
mod blob;
pub use blob::*;
// what a blob store and a table store share.
mod store_backend;
pub use store_backend::*;
// the mark every store reader carries while its content is arriving, the one
// signal a surface waits on before its first paint.
mod loading;
pub use loading::*;
// the one canonical store an app runs from, and the singleton it enforces.
mod package_store;
mod repo_store;
pub use package_store::*;
pub use repo_store::*;

// the analytics types + emission need only serde (which `std` pulls); the store
// persistence inside needs `json` (the TableStore surface) and gates itself.
#[cfg(feature = "std")]
mod analytics;
#[cfg(feature = "json")]
mod document_blob;
#[cfg(feature = "json")]
mod document_store;
// a file computed from declared sources, regenerated when stale: the store
// side of `Provenance`, its producer an action.
#[cfg(feature = "std")]
mod derivation;
#[cfg(feature = "std")]
mod store_ref;
// the uri -> concrete component seam every store construction goes through.
#[cfg(feature = "std")]
mod store_provider;
// tables, beet's one dialect for table-shaped data: rows as canonical json, keyed and
// hashed through DAG-CBOR, so `json` (which brings `dag_cbor`).
#[cfg(feature = "json")]
mod table;
#[cfg(feature = "std")]
pub use analytics::*;
#[cfg(feature = "std")]
pub use derivation::*;
#[cfg(feature = "json")]
pub use document_blob::*;
#[cfg(feature = "json")]
pub(crate) use document_store::*;
#[cfg(feature = "std")]
pub use store_provider::*;
#[cfg(feature = "std")]
pub use store_ref::*;
#[cfg(feature = "json")]
pub use table::*;
// the Worker runtime's ambient env and request conversions, shared by the R2
// blob store and the Durable Object table store
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod worker_env;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub use worker_env::*;

use beet_core::prelude::*;

/// Plugin that registers store types for world serialization, and the
/// observers resolving a scoped store or blob from its nearest ancestor store.
/// no_std-capable: the std backends and the event bus register only with `std`.
#[derive(Default)]
pub struct StorePlugin;

impl Plugin for StorePlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<InMemoryBlobStore>()
			// store-path components: resolve a scoped store / blob from the nearest
			// ancestor store, kept correct as stores churn above them.
			.register_type::<DirPath>()
			.register_type::<BlobPath>()
			// the repo store marker: one canonical store per app, the one an
			// entry loads through, enforced by its own insert hook.
			.register_type::<RepoStore>()
			.add_observer(on_insert_dir_path)
			.add_observer(on_insert_blob_path)
			.add_observer(on_insert_child_of)
			.add_observer(on_insert_store)
			.add_observer(on_remove_store)
			// the two propagation observers marking matching BlobStore / Blob
			// components Changed.
			.add_observer(propagate_blob_store_changes)
			.add_observer(propagate_blob_changes);

		// the consumer -> declared-store relationship, so markup binds a
		// consumer to a declaration (`{StoreRef($analytics)}`), the fs store,
		// and the reactive substrate's global event bus and its drain.
		#[cfg(feature = "std")]
		app.register_type::<FsStore>()
			.register_type::<StoreRef>()
			// a derived file declared beside the action producing it
			.register_type::<Derivation>()
			.register_type::<StoreConsumers>()
			.init_resource::<BlobEventBus>()
			.add_systems(PreUpdate, drain_blob_events)
			.add_observer(add_memory_store_watcher)
			.add_observer(remove_memory_store_watcher);

		// fs watcher lifecycle, one notify debouncer per minimal root of the live
		// `WatchDir` set. Native-only: deno directory watching is unimplemented, so a
		// wasm `FsStore` serves reads through `fs_ext` with no live reload (the
		// `WatchDir` component is inert there). A future wasm watcher wires the same
		// `add_watch_dir`/`remove_watch_dir` seam.
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		app.init_resource::<FsBlobWatchers>()
			.add_observer(add_watch_dir)
			.add_observer(remove_watch_dir);

		// the by-location half of schema resolution: a document naming its
		// schema by path has that schema read out of its own store.
		#[cfg(feature = "json")]
		app.init_resource::<SchemaRegistry>()
			.register_type::<DocumentBlob>()
			.add_systems(
				PreUpdate,
				(
					read_located_schemas
						.run_if(located_schemas_may_be_readable),
					// the documents an app edits: read out of the store onto
					// their own entities, and written back on every edit.
					read_document_blobs,
				),
			)
			.add_systems(PostUpdate, write_document_blobs);

		// wasm localStorage watcher lifecycle (NonSend, owns the JS closure)
		#[cfg(all(target_arch = "wasm32", feature = "std"))]
		app.register_type::<LocalStorageStore>()
			.register_type::<IndexedDbStore>()
			.init_non_send::<LocalStorageBlobWatcher>()
			.add_observer(add_local_storage_store_watcher)
			.add_observer(remove_local_storage_store_watcher);

		// the Cloudflare R2 binding store, registered so a deployed Worker can
		// resolve `<.. R2WorkersStore>` from markup and serialize it.
		#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
		app.register_type::<R2WorkersStore>();

		#[cfg(feature = "aws")]
		app.register_type::<S3Store>()
			.register_type::<DynamoStore>();

		// the database-file store, so a scene declares a local index.
		#[cfg(feature = "sqlite")]
		app.register_type::<SqliteStore>()
			.register_type::<SqliteBlobStore>();

		// partitions as Durable Objects (natively a partition host), so a
		// scene declares `<DurableObjectStore binding="TABLES"/>`
		#[cfg(feature = "json")]
		app.register_type::<DurableObjectStore>()
			// tables over the nearest blob store, following it as it churns
			.register_type::<JsonTableStore>()
			.add_observer(on_insert_json_tables)
			.add_observer(on_insert_json_table_blobs)
			.add_observer(on_parent_json_tables)
			.add_observer(on_remove_json_table_blobs)
			.add_observer(on_remove_json_tables);

		// the http-served store, so a scene declares a remote repo it reads.
		#[cfg(all(feature = "std", feature = "json"))]
		app.register_type::<HttpStore>();
	}
}
