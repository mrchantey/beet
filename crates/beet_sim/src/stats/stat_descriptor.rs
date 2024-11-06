use crate::prelude::*;
use bevy::prelude::*;
use std::any::TypeId;


#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct StatDescriptor {
	pub name: String,
	pub description: String,
	pub emoji: String,
	pub type_id: TypeId,
}
