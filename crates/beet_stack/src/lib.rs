#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
mod content;
mod tools;

/// A prelude for beet_stack, re-exporting the most commonly used items.
pub mod prelude {
	pub use crate::content::*;
	pub use crate::tools::*;
}
