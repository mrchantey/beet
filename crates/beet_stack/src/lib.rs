#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]
#![feature(associated_type_defaults, closure_track_caller)]
#![doc = include_str!("../README.md")]
mod document;
mod input;
mod integrations;
pub mod nodes;
mod parser;
mod router;
mod stack;

mod renderer;

/// A prelude for beet_stack, re-exporting the most commonly used items.
pub mod prelude {
	pub use crate::document::*;
	pub use crate::input::*;
	pub use crate::integrations::*;
	pub use crate::nodes;
	pub use crate::nodes::*;
	pub use crate::parser::*;
	pub use crate::router::*;
	pub use crate::stack::*;
	pub use crate::renderer::*;
	pub use crate::val;
	pub(crate) use beet_tool::prelude::*;
}
/// A module for re-exporting items from other crates.
pub mod exports {
	pub use beet_core::prelude::HashMap;
}
