//! An example of the general pattern used by beet in vanilla bevy
//! Hopefully this makes how beet works a bit clearer

use bevy::prelude::*;

#[derive(Event)]
struct OnRun;
#[derive(Component)]
struct PlayerState(i32);

fn main() {
	let mut app = App::new();

	let entity = app
		.world_mut()
		.spawn(PlayerState(1))
		.observe(my_action)
		.id();
	app.world_mut().flush();
	app.world_mut().entity_mut(entity).trigger(OnRun);
}


fn my_action(trigger: Trigger<OnRun>, query: Query<&PlayerState>) {
	let state = query.get(trigger.entity()).unwrap().0;
	println!("state: {}", state);
}
