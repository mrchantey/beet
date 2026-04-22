use crate::style::*;
use beet_core::prelude::*;

impl TypeTag for f32 {
	const TYPE_TAG: SmolStr = SmolStr::new_static("scalar");
}

impl CssValue for f32 {
	fn to_css_value(&self) -> String { self.to_string() }
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

impl TypeTag for Length {
	const TYPE_TAG: SmolStr = SmolStr::new_static("length");
}

impl CssValue for Length {
	fn to_css_value(&self) -> String { self.to_string() }
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


impl TypeTag for Elevation {
	const TYPE_TAG: SmolStr = SmolStr::new_static("elevation");
}

impl CssValue for Elevation {
	fn to_css_value(&self) -> String {
		[
			self.offset_x.to_css_value(),
			self.offset_y.to_css_value(),
			self.blur_radius.to_css_value(),
			self.spread_radius.to_css_value(),
			self.color.to_css_value(),
		]
		.join(" ")
		.to_string()
	}
}



/// Combined shape token: a corner radius applied to an optional edge.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct Shape {
	pub corner: Token,
	pub edge: ShapeEdge,
}

impl TypeTag for Shape {
	const TYPE_TAG: SmolStr = SmolStr::new_static("shape");
}

impl CssValue for Shape {
	fn to_css_value(&self) -> String {
		// Requires a token store to resolve `self.corner`; handled by the CSS builder.
		todo!("resolve corner token via token store")
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
