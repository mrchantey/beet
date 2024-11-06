use crate::prelude::*;
use bevy::prelude::*;



/// Stats are continuous values that represent real world phenomena.
/// For example they can be used to model health or frequency of events.
/// We deliberately use floating point because it more accurately represents
/// the continuous nature of the real world.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Stat {
	pub value: f32,
	pub id: StatId,
}

impl Stat {
	pub fn new(value: f32, id: StatId) -> Self { Self { value, id } }
}


pub fn stat_plugin(app: &mut App) {
	app.register_type::<Stat>()
		.world_mut()
		.register_component_hooks::<Stat>()
		.on_add(|mut world, entity, _| {
			let map = world.resource::<StatMap>();
			let stat = world
				.get::<Stat>(entity)
				.expect("A StatId must be set before adding a Stat");

			let hexcode = map
				.get(&stat.id)
				.expect("StatId must be in StatMap")
				.emoji_hexcode
				.clone();

			world
				.commands()
				.entity(entity)
				.insert(Emoji::bundle(&hexcode));
		});
}
