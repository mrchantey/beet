#![cfg(not(target_arch = "wasm32"))]
#![feature(async_closure, async_fn_traits)]
pub mod server;

pub mod prelude {
	pub use crate::server::*;
}
