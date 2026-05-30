//! Reactive keyed state: a typed [`DocState<T>`] field stays in sync and reads /
//! writes without touching [`Value`] or `FieldSegment` directly.
use beet_core::prelude::*;
use beet_ui::prelude::*;

fn main() -> Result {
	let mut world = DocumentPlugin::world();
	let count = DocState::<i64>::new("count").with_default(7);

	// the field lives in the entity's document, initialized to the default
	let entity = world
		.spawn((Document::default(), children![count.field()]))
		.id();
	world.update_local();

	count.get(&mut world.entity_mut(entity))?.xpect_eq(7); // ergonomic read
	count.update(&mut world.entity_mut(entity), |n| *n += 1)?; // ergonomic write
	world.update_local();
	count.get(&mut world.entity_mut(entity))?.xpect_eq(8);

	cross_log!("success");
	Ok(())
}
