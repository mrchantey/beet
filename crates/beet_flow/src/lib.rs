#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(deprecated)] // TODO remove deprecated
#![cfg_attr(feature = "reflect", feature(trait_upcasting))]
/// # BeetFlow
///
///
/// Definitions:
///
/// - `Action Entity`: A single global entity that is a trigger
/// 	target
///	- `Node Entity`: The entity representing a node in an action
/// 	graph, it is linked to the `Action Entity` via the [ActionMap].
///
use bevy::app::PluginGroup;
use bevy::app::PluginGroupBuilder;
pub mod action_builder;
pub mod actions;
pub mod events;
pub mod extensions;
pub mod lifecycle;
pub mod observers;
#[cfg(feature = "reflect")]
pub mod reflect;
pub mod tree;

// required for action macros
extern crate self as beet_flow;

pub mod prelude {
	pub use super::BeetFlowPlugin;
	pub use crate::action_builder::*;
	pub use crate::actions::flow::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::global::*;
	pub use crate::actions::misc::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::on_trigger::*;
	#[allow(ambiguous_glob_reexports)]
	pub use crate::actions::*;
	pub use crate::build_observer_hooks;
	pub use crate::events::*;
	pub use crate::extensions::*;
	pub use crate::lifecycle::*;
	pub use crate::observers::*;
	// pub use crate::lifecycle::*;
	#[cfg(feature = "reflect")]
	pub use crate::reflect::*;
	pub use crate::tree::*;
	pub use beet_flow_macros::*;
}


#[derive(Default)]
pub struct BeetFlowPlugin {
	lifecycle_plugin: lifecycle::LifecyclePlugin,
	beet_debug_plugin: lifecycle::BeetDebugPlugin,
}

impl BeetFlowPlugin {
	pub fn new() -> Self { Self::default() }
	/// set [BeetDebugConfig::log_on_start] to true
	pub fn log_on_run(mut self) -> Self {
		self.beet_debug_plugin.log_on_run = true;
		self
	}
}



impl PluginGroup for BeetFlowPlugin {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.lifecycle_plugin)
			.add(self.beet_debug_plugin)
			.add(bevy::log::LogPlugin::default())
	}
}
