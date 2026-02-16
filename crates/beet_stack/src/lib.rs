#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]
#![feature(associated_type_defaults, closure_track_caller)]
#![doc = include_str!("../README.md")]
mod control_flow;
mod document;
#[cfg(feature = "interface")]
mod interface;
pub mod nodes;
mod parsers;
mod renderers;
mod router;
mod stack;
mod tools;

/// A prelude for beet_stack, re-exporting the most commonly used items.
pub mod prelude {
	pub use crate::control_flow::Outcome::Fail;
	pub use crate::control_flow::Outcome::Pass;
	pub use crate::control_flow::*;
	pub use crate::document::*;
	#[cfg(feature = "interface")]
	pub use crate::interface::*;
	pub use crate::nodes;
	pub use crate::nodes::*;
	pub use crate::parsers::*;
	pub use crate::renderers::*;
	pub use crate::router::*;
	pub use crate::stack::*;
	pub use crate::tools::*;
	pub use crate::val;
}
/// A module for re-exporting items from other crates.
pub mod exports {
	pub use beet_core::prelude::HashMap;
}
