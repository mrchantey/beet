//! # Simple Action - Writing Your Own Action
//!
//! Any `async fn` annotated with `#[action]` becomes a reusable action
//! type. The handler receives an [`ActionContext`], from which it can read
//! components off the caller entity via the async world handle.
//!
//! This example also wraps the action with [`trace_action`], middleware
//! that logs on call entry and exit — the replacement for the old
//! `DebugFlowPlugin`.
//!
//! Run with:
//! ```sh
//! cargo run --example simple_action --features action
//! ```
use beet::prelude::*;

/// Greets using the caller entity's [`Name`], then passes.
#[action]
#[derive(Component)]
async fn Greet(cx: ActionContext) -> Result<Outcome> {
	let name = cx
		.caller
		.get(|name: &Name| name.to_string())
		.await
		.unwrap_or_else(|_| "anonymous".to_string());
	info!("hello from {name}");
	Outcome::PASS.xok()
}

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async |world: AsyncWorld| -> Result {
		let greeter = world
			.with(|world: &mut World| {
				world
					.spawn((Name::new("greeter"), trace_action.wrap(Greet)))
					.id()
			})
			.await;
		let outcome = world.entity(greeter).call::<(), Outcome>(()).await?;
		info!("done: {outcome:?}");
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
