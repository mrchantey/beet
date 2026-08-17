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
//! The terminal state is wrapped with [`trace_action`] so its call is logged.
//! The jump nodes are not, and cannot be: an entity holds at most one action,
//! and [`RunNext`] already fills that slot with the [`RunNextAction`] it
//! requires. A colocated wrapper would be an explicit `Action` taking the same
//! slot, which the provider rejects rather than silently losing the jump.
//!
//! Run with:
//! ```sh
//! cargo run --example state_machine --features action
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async |world: AsyncWorld| -> Result {
		let state1 = world
			.with(|world: &mut World| {
				// state2 is the terminal state: it just returns its input.
				let state2 = world
					.spawn((
						Name::new("state2"),
						trace_action.wrap(
							Action::<Outcome, Outcome>::new_pure(
								|cx: ActionContext<Outcome>| cx.input,
							),
						),
					))
					.id();
				// transition forwards to state2.
				let transition = world
					.spawn((Name::new("transition"), RunNext::new(state2)))
					.id();
				// state1 begins the machine and jumps to the transition.
				world
					.spawn((Name::new("state1"), RunNext::new(transition)))
					.id()
			})
			.await;
		let outcome = world
			.entity(state1)
			.call::<Outcome, Outcome>(Outcome::PASS)
			.await?;
		info!("machine finished: {outcome:?}");
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
