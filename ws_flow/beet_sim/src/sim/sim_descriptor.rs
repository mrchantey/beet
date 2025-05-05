use crate::prelude::*;
use bevy::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct SimDescriptor {
	pub stats: Vec<StatDescriptor>,
}
