//! # Scripting - User-Authored Behavior
//!
//! An [`Action`] can run a [`Script`] instead of compiled Rust. The
//! script sees the caller entity's reflected components by their short
//! type name, and any mutations it makes are written back.
//!
//! Here a rhai script increments a `Count` component, then a regular
//! system reports the new value.
//!
//! Run with:
//! ```sh
//! cargo run --example action_scripting --features rhai
//! ```
use beet::prelude::*;

/// A reflected component a script can read and mutate.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Count {
	value: i64,
}

#[beet::main]
async fn main() -> Result {
	let mut world = AsyncPlugin::world();
	world
		.resource_mut::<AppTypeRegistry>()
		.write()
		.register::<Count>();

	let entity = world
		.spawn((
			Count { value: 0 },
			Action::<(), ()>::new_script(Script::rhai("Count.value += 1")),
		))
		.id();

	// run the script three times
	for _ in 0..3 {
		world.entity_mut(entity).call::<(), ()>(()).await?;
	}

	let value = world.entity(entity).get::<Count>().unwrap().value;
	assert_eq!(value, 3);
	println!("final count: {}", value);
	Ok(())
}
