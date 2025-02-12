use bevy::prelude::*;

#[derive(Event)]
struct OnRun;
#[derive(Default, Component)]
struct TriggerCount(i32);

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
		app.world_mut().entity_mut(entity).trigger(OnRun);
	}
	println!("Time: {}", start.elapsed().as_millis());
	// 3000ms
	// assert_eq!(app.world().get::<TriggerCount>(entity).unwrap().0, 1);
}

fn increment(trigger: Trigger<OnRun>, mut query: Query<&mut TriggerCount>) {
	query.get_mut(trigger.entity()).unwrap().as_mut().0 += 1;
}
