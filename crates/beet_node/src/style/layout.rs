use super::*;
use beet_core::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum JustifyContent {
	#[default]
	Start,
	End,
	Center,
	SpaceBetween,
	SpaceEvenly,
	SpaceAround,
}

impl TypeTag for JustifyContent {
	const TYPE_TAG: SmolStr = SmolStr::new_static("justify-content");
}

impl CssValue for JustifyContent {
	fn to_css_value(&self) -> String {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::SpaceBetween => "space-between",
			Self::SpaceEvenly => "space-evenly",
			Self::SpaceAround => "space-around",
		}
		.to_string()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum AlignItems {
	#[default]
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}

impl TypeTag for AlignItems {
	const TYPE_TAG: SmolStr = SmolStr::new_static("align-items");
}

impl CssValue for AlignItems {
	fn to_css_value(&self) -> String {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.to_string()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum AlignSelf {
	#[default]
	Start,
	End,
	Center,
	Stretch,
	Baseline,
}

impl TypeTag for AlignSelf {
	const TYPE_TAG: SmolStr = SmolStr::new_static("align-self");
}

impl CssValue for AlignSelf {
	fn to_css_value(&self) -> String {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.to_string()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum FlexSize {
	#[default]
	Auto,
	Unit(Unit),
	Grow(u16),
	Shrink(u16),
}

impl TypeTag for FlexSize {
	const TYPE_TAG: SmolStr = SmolStr::new_static("flex-size");
}

impl CssValue for FlexSize {
	fn to_css_value(&self) -> String {
		match self {
			Self::Auto => "auto".to_string(),
			Self::Unit(unit) => unit.to_css_value(),
			Self::Grow(n) => n.to_string(),
			Self::Shrink(n) => n.to_string(),
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum Direction {
	#[default]
	Horizontal,
	Vertical,
	ViewportMin,
	ViewportMax,
}

impl TypeTag for Direction {
	const TYPE_TAG: SmolStr = SmolStr::new_static("direction");
}

impl CssValue for Direction {
	fn to_css_value(&self) -> String {
		match self {
			Self::Horizontal => "horizontal",
			Self::Vertical => "vertical",
			Self::ViewportMin => "vmin",
			Self::ViewportMax => "vmax",
		}
		.to_string()
	}
}
