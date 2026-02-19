#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]

mod call_tool;
mod func_tool;
// mod pipe_tool;
mod tool_handler;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::call_tool::*;
	pub use crate::func_tool::*;
	// pub use crate::pipe_tool::*;
	pub use crate::tool_handler::*;
}
