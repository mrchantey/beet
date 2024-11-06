use bevy::prelude::*;
use num_traits::FromPrimitive;
use std::ops::Range;


pub trait NumberStatVal: 'static + Send + Sync + FromPrimitive {}


#[derive(Component, Reflect)]
#[reflect(Default, Component)]
pub struct NumberStat<T: NumberStatVal> {
	pub value: T,
	pub range: Range<T>,
}

impl<T: NumberStatVal> Default for NumberStat<T> {
	/// defaults to 0..1
	fn default() -> Self {
		Self {
			value: T::from_u8(0).unwrap(),
			range: T::from_u8(0).unwrap()..T::from_u8(1).unwrap(),
		}
	}
}


