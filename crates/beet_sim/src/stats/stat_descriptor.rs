use super::StatValue;
use beet_core::prelude::*;
use std::ops::Range;


#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct StatDescriptor {
	pub name: String,
	pub description: String,
	pub emoji_hexcode: String,
	/// The absolute range for this stat
	/// Individual values may have a local subset range
	pub global_range: Range<StatValue>,
	/// Unless overridden this is the default value for this stat
	pub default_value: StatValue,
}


impl StatDescriptor {
	pub fn total_range(&self) -> StatValue {
		StatValue(*self.global_range.end - *self.global_range.start)
	}
}
