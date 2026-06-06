#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

mod extra;
#[cfg(all(feature = "codegen", feature = "std"))]
mod route_codegen;
// The media exchange parses responses into beet_ui scene trees (std-only).
#[cfg(feature = "std")]
mod media;
mod navigate;
mod router;
mod scene_management;
#[cfg(feature = "std")]
mod scene_routes;
#[cfg(feature = "std")]
mod static_export;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::extra::*;
	#[cfg(feature = "std")]
	pub use crate::media::*;
	pub use crate::navigate::*;
	#[cfg(all(feature = "codegen", feature = "std"))]
	pub use crate::route_codegen::*;
	pub use crate::router::*;
	pub use crate::scene_management::*;
	#[cfg(feature = "std")]
	pub use crate::scene_routes::*;
	#[cfg(feature = "std")]
	pub use crate::static_export::*;
}
