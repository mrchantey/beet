use crate::style::*;
use beet_core::prelude::*;

impl AsCssValue for f32 {
	fn as_css_value(&self) -> Result<CssValue> {
		CssValue::expression(self.to_string()).xok()
	}
}
impl AsCssValue for u16 {
	fn as_css_value(&self) -> Result<CssValue> {
		CssValue::expression(self.to_string()).xok()
	}
}


#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum Length {
	Px(f32),
	Rem(f32),
	Percent(f32),
}

impl Default for Length {
	fn default() -> Self { Self::Px(0.0) }
}

impl Length {
	pub fn px(value: f32) -> Self { Self::Px(value) }
	pub fn rem(value: f32) -> Self { Self::Rem(value) }
	pub fn percent(value: f32) -> Self { Self::Percent(value) }
}

impl std::fmt::Display for Length {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Px(value) => write!(f, "{}px", value),
			Self::Rem(value) => write!(f, "{}rem", value),
			Self::Percent(value) => write!(f, "{}%", value),
		}
	}
}

impl AsCssValue for Length {
	fn as_css_value(&self) -> Result<CssValue> {
		CssValue::expression(self.to_string()).xok()
	}
}


#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Elevation {
	pub offset_x: Length,
	pub offset_y: Length,
	pub blur_radius: Length,
	pub spread_radius: Length,
	pub color: Color,
}

impl Default for Elevation {
	fn default() -> Self {
		Elevation {
			offset_x: default(),
			offset_y: default(),
			blur_radius: default(),
			spread_radius: default(),
			color: Color::BLACK,
		}
	}
}

impl AsCssValue for Elevation {
	// box shadow shorthand
	fn as_css_value(&self) -> Result<CssValue> {
		[
			self.offset_x.as_css_value()?.to_string(),
			self.offset_y.as_css_value()?.to_string(),
			self.blur_radius.as_css_value()?.to_string(),
			self.spread_radius.as_css_value()?.to_string(),
			self.color.as_css_value()?.to_string(),
		]
		.into_iter()
		.collect::<Vec<_>>()
		.join(" ")
		.to_string()
		.xmap(CssValue::expression)
		.xok()
	}
}


/// Combined shape token: a corner radius applied to an optional edge.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Shape {
	pub corner: Length,
	pub edge: ShapeEdge,
}

impl AsCssValue for Shape {
	fn as_css_value(&self) -> Result<CssValue> { self.corner.as_css_value() }
}

/// Which edge(s) a shape corner radius applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
pub enum ShapeEdge {
	#[default]
	None,
	Top,
	End,
	Start,
	Bottom,
}
