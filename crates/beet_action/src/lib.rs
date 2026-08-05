// #![deny(missing_docs)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

mod action_plugin;
mod actions;
mod control_flow;
#[cfg(feature = "scripting")]
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
	// mirrors the `scripting::script` module gate: on a wasm quickjs-only build
	// the module has no public surface, so a plain `scripting` gate would warn.
	#[cfg(all(
		feature = "scripting",
		feature = "serde",
		any(
			feature = "rhai",
			all(feature = "quickjs", not(target_arch = "wasm32"))
		)
	))]
	pub use crate::scripting::*;
}

/// Re-exported third-party crates, so downstream consumers (eg `no_std`
/// embedded targets) can reach the [`rhai`] engine through beet rather than
/// declaring their own pinned dependency.
#[allow(unused)]
pub mod exports {
	#[cfg(feature = "rhai")]
	pub use rhai;
	#[cfg(all(feature = "quickjs", not(target_arch = "wasm32")))]
	pub use rquickjs;
}
