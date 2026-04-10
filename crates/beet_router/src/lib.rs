#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

mod media;
mod navigate;
mod router;
mod scene_renderer;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::media::*;
	pub use crate::navigate::*;
	pub use crate::router::*;
	pub use crate::scene_renderer::*;
}
