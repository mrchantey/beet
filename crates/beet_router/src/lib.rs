#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

// the server-to-client websocket channel and dev-mode live reload, native-only
// (a tungstenite listener and an fs watcher).
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
mod client_io;
mod extra;
#[cfg(all(feature = "codegen", feature = "std"))]
mod route_codegen;
// The media exchange parses responses into beet_ui scene trees (std-only).
#[cfg(feature = "std")]
mod media;
mod navigate;
mod router;
// every scene-management module needs `template_serde` (load/save a scene through it).
#[cfg(feature = "template_serde")]
mod scene_management;
#[cfg(feature = "std")]
mod scene_routes;
#[cfg(feature = "std")]
mod static_export;

/// Exports the most commonly used items.
pub mod prelude {
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	pub use crate::client_io::*;
	pub use crate::extra::*;
	#[cfg(feature = "std")]
	pub use crate::media::*;
	pub use crate::navigate::*;
	#[cfg(all(feature = "codegen", feature = "std"))]
	pub use crate::route_codegen::*;
	pub use crate::router::*;
	#[cfg(feature = "template_serde")]
	pub use crate::scene_management::*;
	#[cfg(feature = "std")]
	pub use crate::scene_routes::*;
	#[cfg(feature = "std")]
	pub use crate::static_export::*;
}
