//! The provider-agnostic seam a stack's secrets live behind, and the two
//! providers that serve it: a secrets document on every target, parameter
//! store where the `aws` cli runs.
mod document_secret_store;
mod secret_store;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod ssm_secret_store;
pub use document_secret_store::*;
pub use secret_store::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
pub use ssm_secret_store::*;
