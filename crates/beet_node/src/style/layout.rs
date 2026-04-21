use super::*;
// use crate::prelude::*;
// use beet_core::prelude::*;

#[derive(Default)]
pub enum JustifyContent {
	#[default]
	Start,
	End,
	Center,
	SpaceBetween,
	SpaceEvenly,
	SpaceAround,
}


#[derive(Default)]
pub enum AlignItems {
	#[default]
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}
#[derive(Default)]
pub enum AlignSelf {
	#[default]
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}

#[derive(Default)]
pub enum FlexSize {
	#[default]
	Auto,
	Unit(Unit),
	Grow(u16),
	Shrink(u16),
}




pub enum Direction {
	Horizontal,
	Vertical,
	ViewportMin,
	ViewportMax,
}
