mod bucket;
pub use bucket::*;
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
mod fs_bucket_provider;
#[cfg(target_arch = "wasm32")]
mod local_storage_bucket_provider;
#[cfg(all(not(target_arch = "wasm32"), feature = "tokio"))]
pub use fs_bucket_provider::*;
#[cfg(target_arch = "wasm32")]
pub use local_storage_bucket_provider::*;
