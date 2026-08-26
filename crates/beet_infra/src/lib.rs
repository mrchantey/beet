#![doc = include_str!("../README.md")]

beet_core::test_main!();

// the deploy verbs: every one of them shells out (tofu, docker, ssh, wrangler,
// the aws cli) or drives the S3 SDK, so they are native-only. A wasm consumer
// gets the definitions without the execution.
#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
mod actions;
pub mod bindings;
mod blocks;
// the mail stack's blocks and their declarative identity inputs. Definitions
// only, so a wasm consumer authors a mail stack like any other.
#[cfg(feature = "mail")]
mod mail;
pub mod terra;
mod types;
// the `build-wasm` route action, a cargo/wasm-bindgen/wasm-opt child process.
#[cfg(all(feature = "beet_router", not(target_arch = "wasm32")))]
mod wasm;

// the terraform-schema-to-Rust code generator, a build-time tool that runs
// `tofu providers schema` and writes source files.
#[cfg(all(feature = "bindings_generator", not(target_arch = "wasm32")))]
pub mod bindings_generator;

pub mod prelude {
	#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
	pub use crate::actions::*;
	pub use crate::bindings;
	#[allow(unused)]
	pub use crate::blocks::*;
	#[cfg(feature = "mail")]
	pub use crate::mail::*;
	pub use crate::terra;
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::terra::tofu;
	pub use crate::types::*;
	#[cfg(all(feature = "beet_router", not(target_arch = "wasm32")))]
	pub use crate::wasm::*;
}
