use bevy::ecs::entity::EntityRow;
use bevy::prelude::*;



#[extend::ext]
pub impl Entity {
	/// Creates a new entity using [`Entity::from_raw`]
	/// ## Panics
	/// If the provided value is [`u32::MAX`]
	fn from_num(val: u32) -> Self {
		Entity::from_raw(EntityRow::new(val.try_into().unwrap()))
	}
}
