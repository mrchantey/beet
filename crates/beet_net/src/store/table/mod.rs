//! Tables: beet's one database dialect, see [`TableStore`].
mod blob_table;
mod row;
// the SQL every SQLite table backend runs, whatever engine runs it
mod sql_table;
// the cold-row archive: rows moved into a SQLite file in a bucket
#[cfg(feature = "sqlite")]
mod table_archive;
// many partitions in one process, each a SQLite file, served over the wire
#[cfg(feature = "sqlite")]
mod partition_host;
// one partition's rows and blobs over `Request`/`Response`
mod table_wire;
// partitions as Durable Objects, natively a partition host
mod durable_object_store;
mod table_index;
mod table_provider;
mod table_store;
#[cfg(test)]
pub mod table_test;
pub use durable_object_store::*;
#[cfg(feature = "sqlite")]
pub use partition_host::*;
pub use row::*;
pub use sql_table::*;
#[cfg(feature = "sqlite")]
pub use table_archive::*;
pub use table_index::*;
pub use table_provider::*;
pub use table_store::*;
pub use table_wire::*;
