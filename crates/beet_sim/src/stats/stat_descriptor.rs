use super::StatValue;
use bevy::prelude::*;
use std::ops::Range;


#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct StatDescriptor {
	pub name: String,
	pub description: String,
	pub emoji_hexcode: String,
	/// The absolute range for this stat
	/// Individual values may have a local subset range
	pub global_range: Range<StatValue>,
}
