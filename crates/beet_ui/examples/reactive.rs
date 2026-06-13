//! Reactive keyed state: a typed [`TypedFieldRef<T>`] field stays in sync and
//! reads / writes without touching [`Value`] or `FieldSegment` directly.
use beet_core::prelude::*;

fn main() -> Result {
	let mut world = DocumentPlugin::world();
	let count = TypedFieldRef::<i64>::new("count").with_init(7);

	// the field lives in the entity's document, initialized to the default
	let entity = world
		.spawn((Document::default(), children![count.field()]))
		.id();
	world.update_local();

	count.get(&mut world.entity_mut(entity))?.xpect_eq(7); // ergonomic read
	count.update(&mut world.entity_mut(entity), |n| *n += 1)?; // ergonomic write
	world.update_local();
	count.get(&mut world.entity_mut(entity))?.xpect_eq(8);

	info!("success");
	Ok(())
}
