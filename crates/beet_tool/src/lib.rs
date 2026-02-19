#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]

mod async_tool;
mod call_tool;
mod func_tool;
mod pipe_tool;
mod system_tool;
mod tool_handler;
mod wrap_tool;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::async_tool::*;
	pub use crate::call_tool::*;
	pub use crate::func_tool::*;
	pub use crate::pipe_tool::*;
	pub use crate::system_tool::*;
	pub use crate::tool_handler::*;
	pub use crate::wrap_tool::*;
}
