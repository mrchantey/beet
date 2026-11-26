//! Tables: beet's one dialect for table-shaped data, see [`TableStore`]. A
//! table store holds rows and a blob store bytes, and neither implies the
//! other: each is declared.
// the naive table, json files in any blob store
mod json_table_store;
// tables in process memory, for tests and scratch runtimes
mod in_memory_table_store;
// one partition's rows held whole, shared by the two above
mod partition_rows;
mod row;
// the SQL every SQLite table backend runs, whatever engine runs it
mod sql_table;
// the cold-row archive: rows moved into a SQLite file in a bucket
#[cfg(feature = "sqlite")]
mod table_archive;
// many partitions in one process, each a SQLite file, served over the wire
#[cfg(feature = "sqlite")]
mod partition_host;
// one partition's rows over `Request`/`Response`
mod table_wire;
// partitions as Durable Objects, natively a partition host
mod durable_object_store;
// the Worker side: the object client, and the object over its own storage
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod durable_object_worker;
mod table_index;
mod table_provider;
mod table_store;
#[cfg(test)]
pub mod table_test;
pub use durable_object_store::*;
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub use durable_object_worker::*;
pub use in_memory_table_store::*;
pub use json_table_store::*;
#[cfg(feature = "sqlite")]
pub use partition_host::*;
pub(crate) use partition_rows::*;
pub use row::*;
pub use sql_table::*;
#[cfg(feature = "sqlite")]
pub use table_archive::*;
pub use table_index::*;
pub use table_provider::*;
pub use table_store::*;
pub use table_wire::*;
