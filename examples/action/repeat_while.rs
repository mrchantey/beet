//! # Repeat While - Conditional Looping
//!
//! "Repeat while a condition holds" composes from existing pieces:
//!
//! - [`Repeat`] calls its single child in a loop until the child fails
//! - A [`Sequence`] child whose first node is the loop condition
//! - [`SucceedTimes`] is a condition that passes N times, then fails
//!
//! When the condition fails the sequence fails, which stops the repeat.
//!
//! ```text
//! Repeat
//! └── Sequence
//!     ├── SucceedTimes(2)   condition: passes twice, then fails
//!     └── Log               the work performed each iteration
//! ```
//!
//! Run with:
//! ```sh
//! cargo run --example repeat_while --features action
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
		let root = world
			.with(|world: &mut World| {
				world
					.spawn((Name::new("root"), Repeat::new(), children![(
						Name::new("loop body"),
						Sequence::new(),
						children![
							(Name::new("condition"), SucceedTimes::new(2)),
							(Name::new("work"), Log::new("doing work")),
						],
					)]))
					.id()
			})
			.await;
		let outcome = world.entity(root).call::<(), Outcome>(()).await?;
		info!("loop exited with {outcome:?}");
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
