//! ECS types for interacting with terraform configurations.
mod access_grant;
mod artifacts;
mod deploy_render;
mod deployment;
mod infra_plugin;
mod secret_ref;
mod stack;
mod stack_backend;
mod state_encryption;
// the cli-facing stack verbs, which drive the native tofu `Project`.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod stack_cli;
pub use access_grant::*;
pub use artifacts::*;
pub use deploy_render::*;
pub use deployment::*;
pub use infra_plugin::*;
pub use secret_ref::*;
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
