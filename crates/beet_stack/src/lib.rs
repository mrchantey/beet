#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![deny(missing_docs)]
#![feature(associated_type_defaults, closure_track_caller)]
#![doc = include_str!("../README.md")]
mod content;
mod document;
mod interfaces;
mod tools;
mod stack;

/// A prelude for beet_stack, re-exporting the most commonly used items.
pub mod prelude {
	pub use crate::content::*;
	pub use crate::stack::*;
	pub use crate::document::*;
	pub use crate::interfaces::*;
	pub use crate::tools::*;
	pub use crate::val;
	// reexport for val!
}
/// A module for re-exporting items from other crates.
pub mod exports {
	pub use beet_core::prelude::HashMap;
}
