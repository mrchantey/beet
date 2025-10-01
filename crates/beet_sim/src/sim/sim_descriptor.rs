use crate::prelude::*;
use beet_core::prelude::*;



#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct SimDescriptor {
	pub stats: Vec<StatDescriptor>,
}
