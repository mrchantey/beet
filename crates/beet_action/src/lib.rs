// #![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

mod action_plugin;
mod actions;
mod control_flow;
#[cfg(feature = "rhai")]
mod scripting;

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
	#[cfg(feature = "rhai")]
	pub use crate::scripting::*;
}
