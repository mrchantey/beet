//! The providers a stack's secrets live in, each with its declaration and
//! attach, behind the `SecretStore` seam of `beet_core::secrets`: parameter
//! store on every target (signed requests, no cli), and a secrets document
//! wherever the vault layer is built.
#[cfg(feature = "vault")]
mod document_secret_store;
mod ssm_secret_store;
#[cfg(feature = "vault")]
pub use document_secret_store::*;
pub use ssm_secret_store::*;
