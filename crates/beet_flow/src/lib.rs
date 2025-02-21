#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
#[cfg(feature = "bevy_default")]
pub mod asset_actions;
pub mod continue_run;
pub mod control_flow;
pub mod control_flow_actions;
pub mod tree;
#[allow(unused, reason = "docs")]
use crate::prelude::*;
use bevy::app::PluginGroup;
use bevy::app::PluginGroupBuilder;

/// Include the kitchen sink for beet_flow.
pub mod prelude {
	#[cfg(feature = "bevy_default")]
	pub use crate::asset_actions::*;
	// required for macros to work internally
	pub use super::ActionTag;
	pub use super::BeetFlowPlugin;
	pub use crate as beet_flow;
	pub use crate::continue_run::*;
	pub use crate::control_flow::*;
	pub use crate::control_flow_actions::*;
	pub use crate::tree::*;
	pub use beet_flow_macros::*;
	/// doctest reexports and utilities
	#[cfg(feature = "_doctest")]
	pub mod doctest {
		pub use crate::prelude::*;
		pub use bevy::prelude::*;
		/// for docs, create a world with BeetFlowPlugin
		/// ```
		/// use beet_flow::doctest::*;
		/// let world = world();
		/// ```
		#[cfg(feature = "_doctest")]
		pub fn world() -> World {
			let mut app = App::new();
			app.add_plugins(beet_flow::BeetFlowPlugin::default());
			let world = std::mem::take(app.world_mut());
			world
		}
	}
}


/// All plugins required for a beet_flow application.
/// - [control_flow::control_flow_plugin]
/// - [continue_run::continue_run_plugin]
#[derive(Default)]
pub struct BeetFlowPlugin {
	// lifecycle_plugin: lifecycle::LifecyclePlugin,
}

impl BeetFlowPlugin {}


impl PluginGroup for BeetFlowPlugin {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(control_flow::control_flow_plugin)
			.add(continue_run::continue_run_plugin)
			.build()
	}
}


/// Actions can take many forms, these tags help categorize them.
/// The convention is to add these in a list just after the description
/// of the action, and before the example:
/// ```
/// # use beet_flow::doctest::*;
/// /// ## Tags
/// /// - [LongRunning](ActionTag::LongRunning)
/// /// - [MutateOrigin](ActionTag::MutateOrigin)
/// struct MyAction;
/// ```
pub enum ActionTag {
	/// Actions concerned with control flow, usually
	/// triggering [OnRun] and [OnResult] events.
	ControlFlow,
	/// Actions that use the [Running] component to run
	/// over multiple frames.
	LongRunning,
	/// This action makes global changes to the world.
	MutateWorld,
	/// This action makes changes to the [`origin`](OnRun::origin] entity.
	MutateOrigin,
	/// This action is concerned with providing output to the user or
	/// receiving input.
	InputOutput,
}
