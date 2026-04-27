use crate::style::*;
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

impl AsCssValue for JustifyContent {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::SpaceBetween => "space-between",
			Self::SpaceEvenly => "space-evenly",
			Self::SpaceAround => "space-around",
		}
		.to_string()
		.xmap(CssValue::expression)
		.xok()
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

impl AsCssValue for AlignItems {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.xmap(CssValue::expression)
		.xok()
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

impl AsCssValue for AlignSelf {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum FlexSize {
	#[default]
	Auto,
	Unit(Length),
	Grow(u16),
	Shrink(u16),
}

impl AsCssValue for FlexSize {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Auto => CssValue::expression("auto"),
			Self::Unit(unit) => unit.as_css_value()?,
			Self::Grow(n) => n.as_css_value()?,
			Self::Shrink(n) => n.as_css_value()?,
		}
		.xok()
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

impl AsCssValue for Direction {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Horizontal => "horizontal",
			Self::Vertical => "vertical",
			Self::ViewportMin => "vmin",
			Self::ViewportMax => "vmax",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}
