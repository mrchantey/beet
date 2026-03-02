#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod parsers;
mod renderers;
mod types;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::parsers::*;
	pub use crate::renderers::*;
	pub use crate::types::*;
}
