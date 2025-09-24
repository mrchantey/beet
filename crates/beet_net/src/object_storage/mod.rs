// using aws cli has no cargo dependencies
mod analytics;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
mod aws_cli;
mod bucket;
mod table;
pub use analytics::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "fs"))]
pub use aws_cli::*;
pub use table::*;
mod bucket_item;
mod in_memory_provider;
pub use bucket::*;
pub use bucket_item::*;
pub use in_memory_provider::*;
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
