#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod input;
#[cfg(feature = "net")]
mod navigate;
mod parse;
mod render;
mod types;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::input::*;
	#[cfg(feature = "net")]
	pub use crate::navigate::*;
	pub use crate::parse::*;
	pub use crate::render::*;
	pub use crate::types::*;
}


pub mod exports {
	#[cfg(feature = "tui")]
	pub use bevy_ratatui;
	#[cfg(feature = "tui")]
	pub use ratatui;
}
