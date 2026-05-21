//! # CRUD - Token list operations
//!
//! Demonstrates the [`Push`], [`InsertAt`], [`RemoveAt`] and [`Set`] actions on a
//! [`Token`] holding a `Vec<String>`. The host entity listens on the token,
//! so its [`Value`] component is rebuilt after every mutation.
//!
//! Run with:
//! ```sh
//! cargo run --example crud --features action
//! ```
use beet::prelude::*;

fn todos() -> TokenDefinition<Vec<String>> {
	TokenDefinition::inline(Vec::new())
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	let token = todos();
	let token_ref = TokenRef::new(&token);
	let host = world.spawn(token.into_bundle()).id();

	// the actor entity drives mutations on the token
	let actor = world
		.spawn((
			token_ref,
			Push::<String>::default(),
			InsertAt::<String>::default(),
			RemoveAt,
			Set::<Vec<String>>::default(),
		))
		.id();

	// Create: push three items
	for todo in ["buy milk", "walk dog", "ship beet"] {
		world
			.entity_mut(actor)
			.call::<String, ()>(todo.to_string())
			.await?;
	}
	log_state("after push", &world, host)?;

	// Update: insert at index 1
	world
		.entity_mut(actor)
		.call::<(usize, String), ()>((1, "feed cat".to_string()))
		.await?;
	log_state("after insert at 1", &world, host)?;

	// Delete: remove the first item, capturing what was removed
	let removed = world
		.entity_mut(actor)
		.call::<usize, Option<Value>>(0)
		.await?;
	cross_log!("removed: {removed:?}");
	log_state("after remove at 0", &world, host)?;

	// Replace: overwrite the entire list
	world
		.entity_mut(actor)
		.call::<Vec<String>, ()>(vec!["done".to_string()])
		.await?;
	log_state("after set", &world, host)?;
	Ok(())
}

fn log_state(label: &str, world: &World, host: Entity) -> Result {
	let value = world
		.entity(host)
		.get::<Value>()
		.ok_or_else(|| bevyhow!("missing Value on host"))?;
	cross_log!("{label}: {value}");
	Ok(())
}
