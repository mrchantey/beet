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
		cross_log!("patrolling, step {step}");
	}
	Outcome::PASS.xok()
}

#[beet::main]
async fn main() -> Result {
	let mut world = (MinimalPlugins, AsyncPlugin, ActionPlugin).into_world();
	world
		.spawn((Name::new("root"), Sequence::new(), children![
			(Name::new("patrol"), Patrol),
			(
				Name::new("cooldown"),
				EndInDuration::pass(Duration::from_millis(300)),
			),
			(Name::new("after"), Log::new("patrol complete")),
		]))
		.call::<(), Outcome>(())
		.await?;
	Ok(())
}
