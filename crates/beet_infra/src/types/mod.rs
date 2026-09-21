//! ECS types for interacting with terraform configurations.
mod access_grant;
mod artifacts;
mod build_artifact;
mod deployment;
mod infra_plugin;
// a bucket's per-prefix retention, shared by every provider's bucket block.
mod prefix_expiry;
// where a stack's resources land at each provider, resolved by ancestry.
mod provider_address;
mod resource_scope;
// the class a bucket keeps its objects in, read by its transition rule and by
// a push into it.
mod s3_storage_class;
mod secret_ref;
// the provider-agnostic seam a stack's secrets live behind and its providers
#[cfg(feature = "vault")]
mod secret_store;
mod stack;
mod stack_backend;
mod state_encryption;
// the cli-facing stack verbs, which drive the native tofu `Project`.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod stack_cli;
pub use access_grant::*;
pub use artifacts::*;
pub use build_artifact::*;
pub use deployment::*;
pub use infra_plugin::*;
pub use prefix_expiry::*;
pub use provider_address::*;
pub use resource_scope::*;
pub use s3_storage_class::*;
pub use secret_ref::*;
#[cfg(feature = "vault")]
pub use secret_store::*;
pub use stack::*;
pub use stack_backend::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
pub use stack_cli::*;
pub use state_encryption::*;
// cargo/zigbuild invocations, hence a child process.
#[cfg(not(target_arch = "wasm32"))]
mod cargo_build;
#[cfg(not(target_arch = "wasm32"))]
pub use cargo_build::*;
mod variable;
pub use variable::*;
// mod expression;
// pub use expression::*;
