//! Web utilities for WASM targets
#![allow(async_fn_in_trait)]

// we dont want rust-analyzer loading web-sys when working with native
// so cfg this entire module

#[cfg(target_arch = "wasm32")]
mod dom_utils;
#[cfg(target_arch = "wasm32")]
pub use self::dom_utils::*;
#[cfg(target_arch = "wasm32")]
mod logging;
#[cfg(target_arch = "wasm32")]
pub use self::logging::*;
#[cfg(target_arch = "wasm32")]
mod extensions;
#[cfg(target_arch = "wasm32")]
pub use self::extensions::*;


pub mod prelude {
	#[cfg(target_arch = "wasm32")]
	pub use super::dom_utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use super::extensions::*;
	#[cfg(target_arch = "wasm32")]
	pub use super::logging::*;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_bindgen_futures::spawn_local;
}
