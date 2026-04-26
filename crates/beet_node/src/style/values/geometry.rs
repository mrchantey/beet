use crate::style::*;
use beet_core::prelude::*;

impl AsCssValues for f32 {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		self.to_string().xvec().xok()
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

impl AsCssValues for Length {
	fn as_css_values(&self, _builder: &CssBuilder) -> Result<Vec<String>> {
		self.to_string().xvec().xok()
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

impl AsCssValues for Elevation {
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		// consider not doing shorthand
		[
			self.offset_x.as_css_values(builder)?,
			self.offset_y.as_css_values(builder)?,
			self.blur_radius.as_css_values(builder)?,
			self.spread_radius.as_css_values(builder)?,
			self.color.as_css_values(builder)?,
		]
		.into_iter()
		.flatten()
		.collect::<Vec<_>>()
		.join(" ")
		.to_string()
		.xvec()
		.xok()
	}
}


/// Combined shape token: a corner radius applied to an optional edge.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Shape {
	pub corner: Length,
	pub edge: ShapeEdge,
}

impl AsCssValues for Shape {
	fn as_css_values(&self, builder: &CssBuilder) -> Result<Vec<String>> {
		self.corner.as_css_values(builder)
	}
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
