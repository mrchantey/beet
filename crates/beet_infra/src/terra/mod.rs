//! Terraform config types and utilities.
mod config;
mod ident;
mod misc;
// The two execution modules: `tofu` spawns the opentofu CLI as a child process
// and `project` is the driver that sequences it. Native-only, unlike the config
// *types* beside them, which is what lets a wasm consumer author and serialize a
// stack it cannot itself apply.
#[cfg(not(target_arch = "wasm32"))]
mod project;
#[cfg(not(target_arch = "wasm32"))]
pub mod tofu;
pub use config::*;
pub use ident::*;
pub use misc::*;
#[cfg(not(target_arch = "wasm32"))]
pub use project::*;
