mod bucket;
mod bucket_item;
mod in_memory_provider;
pub use bucket::*;
pub use bucket_item::*;
pub use in_memory_provider::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
mod fs_provider;
#[cfg(target_arch = "wasm32")]
mod local_storage_provider;
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub use fs_provider::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
pub use s3_provider::*;
#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
mod s3_provider;
