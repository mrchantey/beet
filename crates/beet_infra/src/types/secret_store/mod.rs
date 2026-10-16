//! The provider-agnostic seam a stack's secrets live behind, and the two
//! providers that serve it, each with its declaration and attach: a secrets
//! document wherever the vault layer is built, parameter store where the
//! `aws` cli runs (a deployer and a deployed box alike).
#[cfg(feature = "vault")]
mod document_secret_store;
mod secret_store;
mod ssm_secret_store;
#[cfg(feature = "vault")]
pub use document_secret_store::*;
pub use secret_store::*;
pub use ssm_secret_store::*;
