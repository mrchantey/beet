//! # CRUD - document list operations
//!
//! Demonstrates the [`PushField`], [`InsertAtField`], [`RemoveAtField`] and
//! [`SetFieldTyped`] actions on a document field holding a `Vec<String>`. The
//! actor entity references the field on its host document, so the host's list
//! is rebuilt after every mutation.
//!
//! Run with:
//! ```sh
//! cargo run --example crud --features action
//! ```
use beet::prelude::*;

fn todos_field() -> FieldRef {
	FieldRef::new("todos").with_init(Value::List(Vec::new()))
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();

	// the host owns the document the actor mutates
	let host = world.spawn(Document::default()).id();

	// the actor drives mutations on the host's `todos` field
	let actor = world
		.spawn((
			ChildOf(host),
			todos_field(),
			PushField::<String>::default(),
			InsertAtField::<String>::default(),
			RemoveAtField,
			SetFieldTyped::<Vec<String>>::default(),
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
	info!("removed: {removed:?}");
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
		.get::<Document>()
		.ok_or_else(|| bevyhow!("missing Document on host"))?
		.get_field_ref(&[FieldSegment::key("todos")])?;
	info!("{label}: {value}");
	Ok(())
}
