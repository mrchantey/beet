use beet_action::prelude::*;
use beet_core::prelude::*;

/// Markup kick: when the scene loads, [`call`](AsyncEntityActionExt::call) this
/// entity's behavior (the thread's `Sequence`, or the loop wrapping it) and exit
/// the app once it completes.
///
/// Spread it onto a thread's outer root so loading and spawning the scene starts
/// the turn loop, with no hand-written kick glue:
///
/// ```rsx
/// <div {(Repeat, RunThread)}>
///     <div {Thread} {Sequence}> ..actors.. </div>
/// </div>
/// ```
///
/// An endless `Repeat` never completes, so an interactive or auto chat runs until
/// quit; a finite loop (`RepeatTimes`, `RepeatWhileFunctionCallOutput`) exits the
/// process cleanly once it finishes. Calling the **thread**, never an "agent", is
/// the whole point: the `Sequence` is the behavior and runs its actors in order.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunThread;

/// Call every freshly-spawned [`RunThread`] root and write [`AppExit::Success`]
/// once it returns. Mirrors `CallOnSpawn`, but awaits the outcome so a finite
/// thread ends the process; an endless loop simply never resolves.
pub fn run_thread_on_load(
	async_commands: AsyncCommands,
	query: Query<Entity, Added<RunThread>>,
) {
	for entity in query.iter() {
		async_commands.run(async move |world: AsyncWorld| -> Result {
			world.entity(entity).call::<(), Outcome>(()).await?;
			world.write_message(AppExit::Success).await;
			Ok(())
		});
	}
}
