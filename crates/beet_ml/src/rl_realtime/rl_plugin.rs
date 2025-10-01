use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Default)]
pub struct RlPlugin;

impl Plugin for RlPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SessionEntity>();
		let world = app.world_mut();
		world.register_component::<SessionEntity>();
	}
}
