#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard)]
mod actions;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod axum_server;
mod types;

pub mod prelude {
	pub use crate::actions::*;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::axum_server::*;
	pub use crate::types::*;
}


pub mod exports {}
