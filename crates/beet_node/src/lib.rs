#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod document;
mod input;
mod parse;
mod render;
#[cfg(feature = "style")]
pub mod style;
mod types;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::document::*;
	pub use crate::input::*;
	#[cfg(feature = "style")]
	pub use crate::token2;
	pub use crate::parse::*;
	pub use crate::render::*;
	#[cfg(feature = "style")]
	pub use crate::style;
	#[cfg(feature = "style")]
	pub use crate::token;

	pub use crate::types::*;
}


pub mod exports {
	// used by the val! macro
	pub use beet_core::prelude::HashMap;
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	pub use bevy_ratatui;
	#[cfg(all(feature = "tui", not(target_arch = "wasm32")))]
	pub use ratatui;
}
