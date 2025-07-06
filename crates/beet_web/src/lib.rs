#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]


// we dont want rust-analyzer loading web-sys when working with native
// so cfg this entire crate

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
#[cfg(target_arch = "wasm32")]
mod net;
#[cfg(target_arch = "wasm32")]
pub use self::net::*;


#[cfg(target_arch = "wasm32")]
pub mod prelude {
	pub use crate::dom_utils::*;
	pub use crate::extensions::*;
	pub use crate::logging::*;
	pub use crate::net::*;
	pub use wasm_bindgen_futures::spawn_local;
}
