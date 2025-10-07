use bevy::prelude::*;

#[derive(EntityEvent)]
struct Run(Entity);

#[derive(Default, Component)]
struct TriggerCount(i32);

fn increment(trigger: On<GetOutcome>, mut query: Query<&mut TriggerCount>) {
	query.get_mut(trigger.event_target()).unwrap().as_mut().0 += 1;
}

fn main() {
	let mut app = App::new();
	let start = std::time::Instant::now();
	for _ in 0..10_u64.pow(6) {
		let entity = app
			.world_mut()
			.spawn(TriggerCount::default())
			.observe(increment)
			.id();
		app.world_mut().flush();
		app.world_mut().entity_mut(entity).trigger(Run);
		app.world_mut().flush();
	}
	println!("Time: {}", start.elapsed().as_millis());
	// 2200ms
}
