//! # Behavior Tree - Sequence Control Flow
//!
//! Control-flow nodes are just actions that call their children and await
//! the results. A [`Sequence`] runs its children in order, threading the
//! input through each, and stops at the first [`Outcome::Fail`].
//!
//! ## How It Works
//!
//! 1. The root entity gets a [`Sequence`] component
//! 2. Two children each carry a [`Log`] leaf action
//! 3. Calling the root runs child1 then child2; both pass, so the
//!    sequence passes
//!
//! Run with:
//! ```sh
//! cargo run --example behavior_tree --features action
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
					.spawn((Name::new("root"), Sequence::new(), children![
						(Name::new("child1"), Log::new("running child1")),
						(Name::new("child2"), Log::new("running child2")),
					]))
					.id()
			})
			.await;
		let outcome = world.entity(root).call::<(), Outcome>(()).await?;
		info!("sequence finished: {outcome:?}");
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
