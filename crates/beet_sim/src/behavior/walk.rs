use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Walk {}


pub fn walk_plugin(app: &mut App) {
	app.register_type::<Walk>()
		.world_mut()
		.register_component_hooks::<Walk>()
		.on_add(|mut world, cx| {

			// world
			// 	.commands()
			// 	.entity(entity)
			// 	.insert(Emoji::bundle("1F43E"));
		});
}
