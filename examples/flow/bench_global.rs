#![allow(dead_code)]
//! An example of the general pattern used by beet in vanilla bevy
//! Hopefully this makes how beet works a bit clearer
use beet::prelude::*;
use bevy::prelude::*;
use sweet::prelude::EntityWorldMutwExt;

#[action(trigger_count)]
#[derive(Default, Component)]
struct TriggerCount(i32);

fn foobar(_trigger: Trigger<OnRunAction>) {
	println!("foobar");
}

fn trigger_count(trigger: Trigger<OnRun>, mut query: Query<&mut TriggerCount>) {
	query.get_mut(trigger.action).unwrap().as_mut().0 += 1;
}

fn main() {
	let mut app = App::new();
	app.add_plugins(BeetFlowPlugin::default());

	let start = std::time::Instant::now();
	for _ in 0..10_u64.pow(6) {
		let entity = app
			.world_mut()
			.spawn(TriggerCount::default())
			.flush_trigger(OnRun::local())
			.id();
		assert_eq!(app.world().get::<TriggerCount>(entity).unwrap().0, 1);
	}
	println!("Time: {}", start.elapsed().as_millis());
	// 600ms
}
