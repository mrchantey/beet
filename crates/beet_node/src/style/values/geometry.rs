use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;


pub static VISUAL_STYLE_DEFAULT: VisualStyle = VisualStyle::DEFAULT;


/// Visual styling for a cell.
#[derive(Debug, Default, Clone, PartialEq, SetWith, Component)]
pub struct VisualStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attributes
	pub foreground: Option<Color>,
	pub background: Option<Color>,
	pub decoration_color: Option<Color>,
	pub decoration_line: Vec<TextDecoration>,
	pub border_left: Option<Color>,
	pub border_right: Option<Color>,
	pub border_top: Option<Color>,
	pub border_bottom: Option<Color>,
}

impl VisualStyle {
	pub const DEFAULT: Self = Self {
		foreground: None,
		background: None,
		decoration_color: None,
		decoration_line: Vec::new(),
		border_left: None,
		border_right: None,
		border_top: None,
		border_bottom: None,
	};
	pub fn with_border(mut self, color: impl Into<Color>) -> Self {
		let color = Some(color.into());
		self.border_left = color;
		self.border_right = color;
		self.border_top = color;
		self.border_bottom = color;
		self
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TextDecoration {
	/// `text-decoration: underline`
	Underline,
	/// `text-decoration: underline double`
	UnderlineDouble,
	/// `text-decoration: underline wavy`
	UnderlineWavy,
	/// `text-decoration: underline dashed`
	UnderlinDash,
	/// `text-decoration: overline`
	Overline,
	/// `text-decoration: line-through`
	LineThrough,
}

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


#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Length {
	Px(f32),
	Rem(f32),
	Percent(f32),
	// percentage of viewport min
	ViewportMin(f32),
	// percentage of viewport max
	ViewportMax(f32),
}

impl Default for Length {
	fn default() -> Self { Self::DEFAULT }
}

const REM_PIXELS: f32 = 16.0;

impl Length {
	pub const DEFAULT: Self = Self::Px(0.0);
	/// Convert to unit size, mapping 16 pixels to 1 rem
	pub fn into_rem(self, viewport: Vec2) -> f32 {
		match self {
			Self::Px(value) => value / REM_PIXELS,
			Self::Rem(value) => value,
			Self::Percent(value) => value / 100.0 * viewport.y / REM_PIXELS,
			Self::ViewportMin(value) => {
				value / 100.0 * viewport.min_element() / REM_PIXELS
			}
			Self::ViewportMax(value) => {
				value / 100.0 * viewport.max_element() / REM_PIXELS
			}
		}
	}
}

impl std::fmt::Display for Length {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Px(value) => write!(f, "{}px", value),
			Self::Rem(value) => write!(f, "{}rem", value),
			Self::Percent(value) => write!(f, "{}%", value),
			Self::ViewportMin(value) => write!(f, "{}vmin", value),
			Self::ViewportMax(value) => write!(f, "{}vmax", value),
		}
	}
}

impl AsCssValue for Length {
	fn as_css_value(&self) -> Result<CssValue> {
		CssValue::expression(self.to_string()).xok()
	}
}


#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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


css_property!(
	ShapeProps,
	Shape,
	"border-top-left-radius",
	"border-top-right-radius",
	"border-bottom-right-radius",
	"border-bottom-left-radius"
);

/// Combined shape token: a corner radius applied to an optional edge.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Shape {
	/// [`Token`] pointing to the corner radius [`Length`] token.
	pub corner: Token,
	pub edge: ShapeEdge,
}

impl AsCssValues for Shape {
	fn suffixes() -> Vec<CssKey> {
		vec![
			CssKey::static_property("tl"),
			CssKey::static_property("tr"),
			CssKey::static_property("br"),
			CssKey::static_property("bl"),
		]
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		let zero = Length::Px(0.0).as_css_value()?;
		let corner =
			CssVariable::from_token_key(self.corner.key()).xinto::<CssValue>();

		match self.edge {
			ShapeEdge::None => {
				vec![corner.clone(), corner.clone(), corner.clone(), corner]
			}
			ShapeEdge::Top => vec![corner.clone(), corner, zero.clone(), zero],
			ShapeEdge::End => {
				vec![zero.clone(), corner.clone(), corner, zero]
			}
			ShapeEdge::Start => {
				vec![corner.clone(), zero.clone(), zero, corner]
			}
			ShapeEdge::Bottom => {
				vec![zero.clone(), zero, corner.clone(), corner]
			}
		}
		.xok()
	}
}

/// Which edge(s) a shape corner radius applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ShapeEdge {
	#[default]
	None,
	Top,
	End,
	Start,
	Bottom,
}
