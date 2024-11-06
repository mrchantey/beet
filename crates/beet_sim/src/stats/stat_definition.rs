use crate::prelude::*;
use std::ops::Range;




pub struct StatDefinition<T: Stat> {
	pub name: String,
	pub description: String,
	pub emoji: String,
	/// The range within which all implementers of this stat must be
	pub global_range: Range<f32>,
	pub default_value: T,
}

impl<T: Stat> StatDefinition<T> {

}
