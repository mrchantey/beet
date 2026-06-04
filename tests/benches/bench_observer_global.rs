//! Global observer benchmark.
//!
//! Adding and removing observers per entity per spawn can get expensive if done a lot,
//! in that case its better to use global observers, this is a simple example.
//!
//! Run with:
//! ```sh
//! cargo bench --bench bench_observer_global
//! ```

use bevy::prelude::*;

#[derive(EntityEvent)]
struct Run(Entity);

#[derive(Default, Component)]
struct TriggerCount(i32);

fn increment(trigger: On<Run>, mut query: Query<&mut TriggerCount>) {
	query.get_mut(trigger.event_target()).unwrap().as_mut().0 += 1;
}

fn main() {
	let mut app = App::new();
	app.add_observer(increment);
	let start = std::time::Instant::now();
	for _ in 0..10_u64.pow(6) {
		let entity = app.world_mut().spawn(TriggerCount::default()).id();
		app.world_mut().flush();
		app.world_mut().entity_mut(entity).trigger(Run);
		app.world_mut().flush();
	}
	println!("Time: {}", start.elapsed().as_millis());
	// 200ms
}
