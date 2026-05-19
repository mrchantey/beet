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
//! cargo run --example action_simple_action --features action
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
	cross_log!("hello from {name}");
	Outcome::PASS.xok()
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let outcome = world
		.spawn((Name::new("greeter"), trace_action.wrap(Greet)))
		.call::<(), Outcome>(())
		.await?;
	cross_log!("done: {outcome:?}");
	Ok(())
}
