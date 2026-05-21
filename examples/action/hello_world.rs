//! # Hello World - Entities as Functions
//!
//! `beet_action` lets an entity behave like a callable function. An
//! [`Action`] component wraps a handler; calling it runs the handler with
//! an input and yields an output.
//!
//! ## How It Works
//!
//! 1. The `#[action]` macro turns a plain function into an action type
//! 2. Spawning that type inserts the matching `Action` component
//! 3. `call` invokes the handler and awaits its result
//!
//! Run with:
//! ```sh
//! cargo run --example hello_world --features action
//! ```
use beet::prelude::*;

/// Greets the given name.
#[action(pure)]
#[derive(Component)]
fn Greet(name: String) -> String { format!("Hello, {name}!") }


#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let message = world
		.spawn(Greet)
		.call::<String, String>("world".to_string())
		.await?;
	cross_log!("{message}");
	Ok(())
}
