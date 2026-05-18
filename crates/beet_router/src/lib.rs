
beet_core::test_main!();

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
