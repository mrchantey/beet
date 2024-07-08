use bevy::prelude::*;
use crate::prelude::*;


/// Adds types and systems for beet observers
pub struct BeetObserverPlugin;


impl Plugin for BeetObserverPlugin {
	fn build(&self, app: &mut App) {

		app
    .register_type::<LongRun>()
    .register_type::<SequenceFlow>()
    .register_type::<EndOnRun>()
    .register_type::<RunOnSpawn>()
		
		/*-*/;
		let world = app.world_mut();
		world.observe(bubble_run_result);
		world.observe(trigger_run_on_spawn);

	}
}
