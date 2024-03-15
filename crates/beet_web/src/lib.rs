#![cfg(target_arch = "wasm32")]
#![allow(async_fn_in_trait)]
#![feature(async_closure, async_fn_traits)]

pub mod app;
pub mod bee;
pub mod dom;

// currently required for action_list! macro to work
extern crate beet_core as beet;

pub mod prelude {
	pub use crate::app::*;
	pub use crate::bee::*;
	pub use crate::dom::*;
}
