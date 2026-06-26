//! A headless behaviour-tree example, so the same `Sequence` control flow that
//! drives an agent's turns can drive a robot's movements, run from markup with no
//! window or renderer:
//!
//! ```bsx
//! <div {Behavior} {RunOnLoad}>
//!   <TwoWheelDrive left=1. right=1./>
//!   <TwoWheelDrive left=0. right=1./>
//!   <TwoWheelDrive left=1. right=1./>
//! </div>
//! ```
//!
//! `{Behavior}` is a non-generic marker for a `Sequence<(), ()>` (a bare generic
//! `{Sequence}` tag does not resolve by name);
//! [`RunOnLoad`](beet_net::prelude::RunOnLoad) runs the entity's behaviour on load,
//! exiting once the sequence completes. `<TwoWheelDrive>` lowers to a [`Log`] action
//! printing its wheel speeds; swap it for a real long-running motor action to move a
//! body.
use beet_action::prelude::*;
use beet_core::prelude::*;

/// `<TwoWheelDrive left=.. right=../>` — a stand-in motor action that logs its
/// wheel speeds (headless). Lowers to a [`Log`] so it composes into any control
/// flow as an `Action<(), Outcome>`.
#[template]
pub fn TwoWheelDrive(left: f32, right: f32) -> impl Bundle {
	Log::new(format!("TwoWheelDrive: left={left} right={right}"))
}

/// Registers the headless behaviour example types, so a `main.bsx` declaring
/// `<TwoWheelDrive>` resolves (the `{Behavior}` sequence marker is registered by
/// `AgentExamplesPlugin`, the `{RunOnLoad}` load verb by the server plugin).
pub struct BehaviorExamplesPlugin;

impl Plugin for BehaviorExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.register_template::<TwoWheelDrive>();
	}
}
