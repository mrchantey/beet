#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// #![deny(missing_docs)]

mod control_flow;
mod actions;

/// Exports the most commonly used items.
pub mod prelude {
	// Pass and Fail are top level variant exports, like `Ok`, `Err`
	pub const FAIL: Outcome<(), ()> = crate::control_flow::Outcome::FAIL;
	pub const PASS: Outcome<(), ()> = crate::control_flow::Outcome::PASS;
	pub use crate::control_flow::Outcome::Fail;
	pub use crate::control_flow::Outcome::Pass;
	pub use crate::control_flow::*;
	pub use crate::actions::*;
}
