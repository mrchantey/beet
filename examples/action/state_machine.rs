//! # State Machine - Graph Transitions via `RunNext`
//!
//! Behavior trees flow through parent/child links. A state machine instead
//! jumps between arbitrary entities. [`RunNext`] is that jump: when called
//! it threads its input into another entity and returns that entity's
//! result, regardless of where the target sits in the hierarchy.
//!
//! ```text
//! state1 ──> transition ──> state2
//! ```
//!
//! Each node is wrapped with [`trace_action`] so the traversal is logged.
//!
//! Run with:
//! ```sh
//! cargo run --example state_machine --features action
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();

	// state2 is the terminal state: it just returns its input.
	let state2 = world
		.spawn((
			Name::new("state2"),
			trace_action.wrap(Action::<Outcome, Outcome>::new_pure(
				|cx: ActionContext<Outcome>| cx.input,
			)),
		))
		.id();

	// transition forwards to state2.
	let transition = world
		.spawn((
			Name::new("transition"),
			RunNext::new(state2),
			trace_action.wrap(RunNextAction::<Outcome>::default()),
		))
		.id();

	// state1 begins the machine and jumps to the transition.
	let outcome = world
		.spawn((
			Name::new("state1"),
			RunNext::new(transition),
			trace_action.wrap(RunNextAction::<Outcome>::default()),
		))
		.call::<Outcome, Outcome>(Outcome::PASS)
		.await?;
	cross_log!("machine finished: {outcome:?}");
	Ok(())
}
