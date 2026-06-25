//! # Scripting - User-Authored Behavior
//!
//! A [`Script`] turns an entity into a pure `Input -> Output` action
//! whose body is rhai source instead of compiled Rust. The action input
//! is bound to a variable named `input`; the script's final expression
//! is the output.
//!
//! Run with:
//! ```sh
//! cargo run --example scripting --features rhai_serde
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

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), AsyncPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async |world: AsyncWorld| -> Result {
		// numeric transform: increment the input. `Script` is pure data, so pair
		// it with a `ScriptAction` to make the entity a callable action.
		let entity = world
			.with(|world: &mut World| {
				world
					.spawn((
						Script::<i64, i64>::rhai("input + 1"),
						ScriptAction::<i64, i64>::default(),
					))
					.id()
			})
			.await;
		let count = world.entity(entity).call::<i64, i64>(41).await?;
		info!("count is now {count}");

		// string transform: greet the input
		let entity = world
			.with(|world: &mut World| {
				world
					.spawn((
						Script::<String, String>::rhai(r#""hello " + input"#),
						ScriptAction::<String, String>::default(),
					))
					.id()
			})
			.await;
		let greeting = world
			.entity(entity)
			.call::<String, String>("world".to_string())
			.await?;
		info!("{greeting}");

		// struct transform: mutate a single field, return the struct
		let entity = world
			.with(|world: &mut World| {
				world
					.spawn((
						Script::<Player, Player>::rhai("input.score += 100; input"),
						ScriptAction::<Player, Player>::default(),
					))
					.id()
			})
			.await;
		let player = world
			.entity(entity)
			.call::<Player, Player>(Player {
				name: "ada".to_string(),
				score: 1,
			})
			.await?;
		info!("{} now has score {}", player.name, player.score);

		world.write_message(AppExit::Success).await;
		Ok(())
	});
}
