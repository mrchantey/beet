#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

#[cfg(all(feature = "codegen", feature = "std"))]
mod route_codegen;
mod extra;
// The media exchange parses responses into beet_ui scene trees (std-only).
#[cfg(feature = "std")]
mod media;
mod navigate;
mod router;
#[cfg(feature = "std")]
mod scene_routes;
#[cfg(feature = "std")]
mod static_export;

/// Exports the most commonly used items.
pub mod prelude {
	#[cfg(all(feature = "codegen", feature = "std"))]
	pub use crate::route_codegen::*;
	pub use crate::extra::*;
	#[cfg(feature = "std")]
	pub use crate::media::*;
	pub use crate::navigate::*;
	pub use crate::router::*;
	#[cfg(feature = "std")]
	pub use crate::scene_routes::*;
	#[cfg(feature = "std")]
	pub use crate::static_export::*;
}
