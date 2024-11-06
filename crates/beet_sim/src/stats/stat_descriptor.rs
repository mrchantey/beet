use bevy::prelude::*;
use std::any::TypeId;


#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct StatDescriptor {
	pub name: String,
	pub description: String,
	pub emoji_hexcode: String,
	/// The id of the stat, ie for f32 stat `type_id::of::<f32>()`
	pub type_id: TypeId,
}
