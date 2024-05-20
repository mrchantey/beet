#![cfg(not(target_arch = "wasm32"))]
// no async_closures, too unstable
#![feature(async_fn_traits)]
pub mod server;

pub mod prelude {
	pub use crate::server::*;
}
