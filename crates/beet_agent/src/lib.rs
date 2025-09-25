#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

// TODO wasm with bevy 0.17
pub mod realtime;
#[cfg(not(target_arch = "wasm32"))]
mod session;

#[path = "session/content.rs"]
#[cfg(target_arch = "wasm32")]
mod content;


pub mod prelude {

	#[cfg(target_arch = "wasm32")]
	pub use crate::content::*;
	pub use crate::realtime;
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::session::*;
}
