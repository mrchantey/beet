use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


pub fn procedural_animation_plugin(app: &mut App) {
	app.add_plugins(
		ActionPlugin::<InsertOnRun<PlayProceduralAnimation>>::default(),
	);
}
