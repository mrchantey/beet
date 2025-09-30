#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![deny(missing_docs)]
// #![doc = include_str!("../README.md")]
#[cfg(feature = "bevy_default")]
#[allow(unused, reason = "docs")]
use crate::prelude::*;

mod events;

/// Include the kitchen sink for beet_flow.
pub mod prelude {
	pub use crate::events::*;
	pub use beet_flow_macros::*;
}
/// doctest reexports and utilities
#[cfg(feature = "_doctest")]
pub mod doctest {}


/// All plugins required for a beet_flow application.
/// The primary role that this plugin plays is as a kind of
/// observer router, ensuring the OnRun and OnResult events are propagated
/// correctly.
/// - [control_flow::control_flow_plugin]
/// - [continue_run::continue_run_plugin]
#[derive(Default)]
pub struct BeetFlowPlugin {
	// lifecycle_plugin: lifecycle::LifecyclePlugin,
}
