
beet_core::test_main!();

#[cfg(feature = "codegen")]
mod route_codegen;
mod media;
mod navigate;
mod router;
mod scene_renderer;
mod static_export;

/// Exports the most commonly used items.
pub mod prelude {
	#[cfg(feature = "codegen")]
	pub use crate::route_codegen::*;
	pub use crate::media::*;
	pub use crate::navigate::*;
	pub use crate::router::*;
	pub use crate::scene_renderer::*;
	pub use crate::static_export::*;
}
