//! ECS types for interacting with terraform configurations.
mod access_grant;
mod artifacts;
mod infra_plugin;
mod resource_scope;
mod stack;
mod stack_backend;
// the cli-facing stack verbs, which drive the native tofu `Project`.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod stack_cli;
pub use access_grant::*;
pub use artifacts::*;
pub use infra_plugin::*;
pub use resource_scope::*;
pub use stack::*;
pub use stack_backend::*;
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
pub use stack_cli::*;
// cargo/zigbuild invocations, hence a child process.
#[cfg(not(target_arch = "wasm32"))]
mod cargo_build;
#[cfg(not(target_arch = "wasm32"))]
pub use cargo_build::*;
mod variable;
pub use variable::*;
// mod expression;
// pub use expression::*;
