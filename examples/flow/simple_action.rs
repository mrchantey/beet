//! # Simple Action - Custom Behavior Implementation
//!
//! This example shows how to create a custom action using the `#[action]` macro.
//!
//! ## Key Concepts
//!
//! - **#[action]**: Macro that sets up an observer to handle `GetOutcome` events
//! - **On<GetOutcome>**: The trigger event passed to action handlers
//! - **ev.target()**: Returns the entity that received the event
//! - **expect_action**: Helper for error messages when queries fail
//!
//! ## How Actions Work
//!
//! 1. The `#[action(log_on_run)]` attribute registers the `log_on_run` function
//!    as an observer for `GetOutcome` events
//! 2. When `GetOutcome` is triggered on an entity with `LogOnRun`, the observer fires
//! 3. The handler queries for its component data and performs its logic
//!
//! ## Creating Your Own Actions
//!
//! ```ignore
//! #[action(my_handler)]
//! #[derive(Component)]
//! struct MyAction { /* fields */ }
//!
//! fn my_handler(
//!     ev: On<GetOutcome>,
//!     mut commands: Commands,
//!     query: Query<&MyAction>,
//! ) {
//!     let action = query.get(ev.target()).expect("action exists");
//!     // ... do something ...
//!     commands.entity(ev.target()).trigger_target(Outcome::Pass);
//! }
//! ```
use beet::prelude::*;

#[action(log_on_run)]
#[derive(Component)]
struct LogOnRun(pub String);

fn log_on_run(ev: On<GetOutcome>, query: Query<&LogOnRun>) {
	let name = query
		.get(ev.target())
		// Common pattern for getting an action component -
		// it should never be missing if the observer fired
		.expect(&expect_action::to_have_action(&ev));
	println!("running: {}", name.0);
}

fn main() {
	App::new()
		.add_plugins(ControlFlowPlugin::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.trigger_target(GetOutcome)
		.flush();
	println!("done!");
}
