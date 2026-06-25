//! # Long Running - Multi-Step Actions as Futures
//!
//! In the old event model a long-running action needed a `Running`
//! component and a timer system. With async actions, "long running" is
//! simply a future that takes a while to resolve — the handler `await`s
//! whatever it needs and the rest of the tree waits for it.
//!
//! Here `Patrol` loops several times with a delay between steps, then a
//! [`EndInDuration`] cooldown demonstrates a timer-driven leaf.
//!
//! Run with:
//! ```sh
//! cargo run --example long_running --features action
//! ```
use beet::prelude::*;
use std::time::Duration;

/// Patrols for a few steps, sleeping between each, then passes.
#[action]
#[derive(Component)]
async fn Patrol(cx: ActionContext) -> Result<Outcome> {
	let _ = cx;
	for step in 1..=5 {
		time_ext::sleep(Duration::from_millis(200)).await;
		info!("patrolling, step {step}");
	}
	Outcome::PASS.xok()
}

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), ActionPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async |world: AsyncWorld| -> Result {
		let root = world
			.with(|world: &mut World| {
				world
					.spawn((Name::new("root"), Sequence::new(), children![
						(Name::new("patrol"), Patrol),
						(
							Name::new("cooldown"),
							EndInDuration::pass(Duration::from_millis(300)),
						),
						(Name::new("after"), Log::new("patrol complete")),
					]))
					.id()
			})
			.await;
		world.entity(root).call::<(), Outcome>(()).await?;
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
