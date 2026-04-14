#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]

mod action_plugin;
mod actions;
mod control_flow;

/// Exports the most commonly used items.
pub mod prelude {
	// Pass and Fail are top level variant exports, like `Ok`, `Err`
	pub const FAIL: Outcome<(), ()> = crate::control_flow::Outcome::FAIL;
	pub const PASS: Outcome<(), ()> = crate::control_flow::Outcome::PASS;
	pub use crate::action_plugin::*;
	pub use crate::actions::*;
	pub use crate::control_flow::Outcome::Fail;
	pub use crate::control_flow::Outcome::Pass;
	pub use crate::control_flow::*;
}
