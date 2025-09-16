#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

// TODO wasm with bevy 0.17
#[cfg(not(target_arch = "wasm32"))]
mod core;
pub mod realtime;


pub mod prelude {

	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::core::*;
	pub use crate::realtime;
}
