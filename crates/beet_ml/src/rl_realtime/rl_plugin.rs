use crate::prelude::*;
use bevy::prelude::*;


#[derive(Default)]
pub struct RlPlugin;

impl Plugin for RlPlugin {
	fn build(&self, app: &mut App) {
		let world = app.world_mut();
		world.init_component::<SessionEntity>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();

		registry.register::<SessionEntity>();
	}
}
