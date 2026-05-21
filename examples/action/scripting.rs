//! # Scripting - User-Authored Behavior
//!
//! A [`Script`] turns an entity into a pure `Input -> Output` action
//! whose body is rhai source instead of compiled Rust. The action input
//! is bound to a variable named `input`; the script's final expression
//! is the output.
//!
//! Run with:
//! ```sh
//! cargo run --example scripting --features scripting
//! ```
use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// An arbitrary struct passed through a script, which mutates one field.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Player {
	name: String,
	score: i64,
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();

	// numeric transform: increment the input
	let count = world
		.spawn(Script::<i64, i64>::rhai("input + 1"))
		.call::<i64, i64>(41)
		.await?;
	cross_log!("count is now {count}");

	// string transform: greet the input
	let greeting = world
		.spawn(Script::<String, String>::rhai(r#""hello " + input"#))
		.call::<String, String>("world".to_string())
		.await?;
	cross_log!("{greeting}");

	// struct transform: mutate a single field, return the struct
	let player = world
		.spawn(Script::<Player, Player>::rhai("input.score += 100; input"))
		.call::<Player, Player>(Player {
			name: "ada".to_string(),
			score: 1,
		})
		.await?;
	cross_log!("{} now has score {}", player.name, player.score);

	Ok(())
}
