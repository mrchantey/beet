#![cfg(target_arch = "wasm32")]
#![allow(async_fn_in_trait)]
#![feature(async_closure, async_fn_traits)]

pub mod app;
pub mod dom;

pub mod prelude {
	pub use crate::app::*;
	pub use crate::dom::*;
}
