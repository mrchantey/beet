#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(deprecated)]
// #![deny(missing_docs)]
#![doc = include_str!("../README.md")]
pub mod continue_run;
pub mod control_flow;
pub mod control_flow_actions;
pub mod tree;
#[allow(unused, reason = "docs")]
use crate::prelude::*;
use bevy::app::PluginGroup;
use bevy::app::PluginGroupBuilder;

pub mod prelude {
	// required for macros to work internally
	pub use super::*;
	pub use crate as beet_flow;
	pub use crate::continue_run::*;
	pub use crate::control_flow::*;
	pub use crate::control_flow_actions::*;
	pub use crate::tree::*;
	pub use beet_flow_macros::*;
	// allow flush_trigger in examples
	// #[cfg(feature = "sweet")]
	// pub use sweet::prelude::CoreWorldExtSweet;
	// // allow flush_trigger in examples
	// #[cfg(feature = "sweet")]
	// pub use sweet::prelude::EntityWorldMutwExt;
}


#[derive(Default)]
pub struct BeetFlowPlugin {
	// lifecycle_plugin: lifecycle::LifecyclePlugin,
}

impl BeetFlowPlugin {
	pub fn new() -> Self { Self::default() }
}


impl PluginGroup for BeetFlowPlugin {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(control_flow::control_flow_plugin)
			.add(continue_run::continue_run_plugin)
			.build()
	}
}


/// Actions can take many forms, these tags help categorize them.
pub enum ActionTag {
	/// Actions concerned with control flow, usually
	/// triggering [OnRun] and [OnResult] events.
	ControlFlow,
	/// Actions that use the [Running] component to run
	/// over multiple frames.
	LongRunning,
}


