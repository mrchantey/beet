#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![deny(missing_docs)]

mod actions;
mod types;

/// Prelude module re-exporting commonly used items.
pub mod prelude {
	pub use crate::actions::*;
	pub use crate::types::*;
}
