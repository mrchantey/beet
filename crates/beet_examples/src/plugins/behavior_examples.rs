//! A headless behaviour-tree example, so the same `Sequence` control flow that
//! drives an agent's turns can drive a robot's movements, run from markup with no
//! window or renderer:
//!
//! ```bsx
//! <RunSequence>
//!   <TwoWheelDrive left=1. right=1./>
//!   <TwoWheelDrive left=0. right=1./>
//!   <TwoWheelDrive left=1. right=1./>
//! </RunSequence>
//! ```
//!
//! `{Behavior}` is a non-generic marker for a `Sequence<(), ()>` (a bare generic
//! `{Sequence}` tag does not resolve by name); [`RunOnLoad`] runs the entity's
//! behaviour to completion on load and then exits. `<TwoWheelDrive>` lowers to a
//! [`Log`] action printing its wheel speeds; swap it for a real long-running motor
//! action to move a body.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Boot verb for a plain behaviour tree: on load it runs the entity's
/// `Action<(), Outcome>` (eg a `Sequence`) to completion, then exits. The
/// behaviour-tree counterpart of `CreateThread`, without the conversation store.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(BootOnLoad)]
#[require(Action<Boot, Response> = Action::new_async_local(run_on_boot))]
pub struct RunOnLoad;

/// The `Action<Boot, Response>` boot slot [`RunOnLoad`] installs: run the
/// behaviour to completion, then resolve with an empty response so the one-shot
/// exits.
async fn run_on_boot(cx: ActionContext<Boot>) -> Result<Response> {
	cx.caller.clone().call::<(), Outcome>(()).await?;
	Response::ok().xok()
}

/// `<TwoWheelDrive left=.. right=../>` — a stand-in motor action that logs its
/// wheel speeds (headless). Lowers to a [`Log`] so it composes into any control
/// flow as an `Action<(), Outcome>`.
#[template]
pub fn TwoWheelDrive(left: f32, right: f32) -> impl Bundle {
	Log::new(format!("TwoWheelDrive: left={left} right={right}"))
}

/// Registers the headless behaviour example types, so a `main.bsx` declaring
/// `{RunOnLoad}`/`<TwoWheelDrive>` resolves (the `{Behavior}` sequence marker is
/// registered by `AgentExamplesPlugin`).
pub struct BehaviorExamplesPlugin;

impl Plugin for BehaviorExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<RunOnLoad>()
			.register_template::<TwoWheelDrive>();
	}
}
