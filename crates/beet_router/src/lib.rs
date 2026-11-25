#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

// the reusable stack-of-cards machinery (the HyperCard model): a `CardDeck`
// router plus its `CardStackPlugin`. Self-contained; the navigate/router core
// stays card-agnostic.
mod card_stack;
// the server-to-client websocket channel and dev-mode live reload, native-only
// (a tungstenite listener and an fs watcher).
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
mod client_io;
// the no-code "type-checker": a post-build validation pass over the render tree.
// std-only, it needs the `beet_ui` `RuleSet`/`ElementQuery`.
#[cfg(feature = "std")]
mod diagnostics;
mod extra;
// finding, building and running an entry document: the half of a beet binary
// that is not its capabilities. Store-backed (`std`) and document-building
// (for the entry's body and the `BeetSceneRoot` a watched rebuild swaps).
#[cfg(feature = "std")]
pub mod launch;
mod navigate;
#[cfg(all(feature = "codegen", feature = "std"))]
mod route_codegen;
mod router;
// every scene-management module loads or saves a scene.
#[cfg(feature = "scene")]
mod scene_management;
#[cfg(feature = "std")]
mod scene_routes;
#[cfg(feature = "std")]
mod static_export;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::card_stack::*;
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	pub use crate::client_io::*;
	#[cfg(feature = "std")]
	pub use crate::diagnostics::*;
	#[cfg(feature = "std")]
	pub use crate::extra::*;
	#[cfg(feature = "std")]
	pub use crate::launch::entry_build;
	#[cfg(feature = "std")]
	pub use crate::launch::*;
	pub use crate::navigate::*;
	#[cfg(all(feature = "codegen", feature = "std"))]
	pub use crate::route_codegen::*;
	pub use crate::router::*;
	#[cfg(feature = "scene")]
	pub use crate::scene_management::*;
	#[cfg(feature = "std")]
	pub use crate::scene_routes::*;
	#[cfg(feature = "std")]
	pub use crate::static_export::*;
}
