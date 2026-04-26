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

impl AsCssValues for JustifyContent {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::SpaceBetween => "space-between",
			Self::SpaceEvenly => "space-evenly",
			Self::SpaceAround => "space-around",
		}
		.to_string()
		.xvec()
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

impl AsCssValues for AlignItems {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.to_string()
		.xvec()
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

impl AsCssValues for AlignSelf {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		match self {
			Self::Start => "start",
			Self::End => "end",
			Self::Center => "center",
			Self::Stretch => "stretch",
			Self::Baseline => "baseline",
		}
		.to_string()
		.xvec()
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

impl AsCssValues for FlexSize {
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		match self {
			Self::Auto => "auto".to_string().xvec(),
			Self::Unit(unit) => unit.as_css_values(builder)?,
			Self::Grow(n) => n.to_string().xvec(),
			Self::Shrink(n) => n.to_string().xvec(),
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

impl AsCssValues for Direction {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		match self {
			Self::Horizontal => "horizontal",
			Self::Vertical => "vertical",
			Self::ViewportMin => "vmin",
			Self::ViewportMax => "vmax",
		}
		.to_string()
		.xvec()
		.xok()
	}
}
