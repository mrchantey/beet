use beet_core::prelude::*;
use bitflags::bitflags;

use crate::style::AsCssValue;
use crate::style::CssValue;

pub static VISUAL_STYLE_DEFAULT: VisualStyle = VisualStyle::DEFAULT;


/// Visual styling for a cell.
#[derive(Debug, Default, Clone, PartialEq, SetWith, Component)]
pub struct VisualStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attribute
	pub foreground: Option<Color>,
	pub background: Option<Color>,
	/// Color of underlines, overlines etc
	pub decoration_color: Option<Color>,
	pub text_style: TextStyle,
	pub text_align: TextAlign,
}

impl VisualStyle {
	pub const DEFAULT: Self = Self {
		foreground: None,
		background: None,
		decoration_color: None,
		text_style: TextStyle::empty(),
		text_align: TextAlign::Left,
	};

	/// Returns true if the foreground color is set and has an alpha value of 0.5 or less.
	pub fn dim_foreground(&self) -> bool {
		match self.foreground {
			Some(color) => color.alpha() <= 0.5,
			None => false,
		}
	}
}


/// Text alignment within a cell's content area.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TextAlign {
	#[default]
	Left,
	Center,
	Right,
}

impl AsCssValue for TextAlign {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Left => "left",
			Self::Center => "center",
			Self::Right => "right",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}

#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DecorationLine {
	underline: bool,
	overline: bool,
	line_through: bool,
}

impl AsCssValue for DecorationLine {
	fn as_css_value(&self) -> Result<CssValue> {
		let mut parts = Vec::new();
		if self.underline {
			parts.push("underline");
		}
		if self.overline {
			parts.push("overline");
		}
		if self.line_through {
			parts.push("line-through");
		}
		if parts.is_empty() {
			return "none".xmap(CssValue::expression).xok();
		}
		let value = parts.join(" ");
		value.xmap(CssValue::expression).xok()
	}
}

#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DecorationStyle {
	#[default]
	Solid,
	Wavy,
	Double,
	Dash,
}

impl AsCssValue for DecorationStyle {
	fn as_css_value(&self) -> Result<CssValue> {
		match self {
			Self::Solid => "solid",
			Self::Wavy => "wavy",
			Self::Double => "double",
			Self::Dash => "dashed",
		}
		.xmap(CssValue::expression)
		.xok()
	}
}


bitflags! {
	/// Text styling modifiers combining CSS text-decoration and TUI modifier concepts.
	#[repr(transparent)]
	#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
	#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
	#[reflect(opaque)]
	#[reflect(Hash, Clone, PartialEq, Debug, Default)]
	#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
	pub struct TextStyle: u16 {
		/// Bold text
		const BOLD = 1 << 0;
		/// Italic text
		const ITALIC = 1 << 1;
		/// Slowly blinking text
		const BLINK = 1 << 2;
		/// Rapidly blinking text
		const RAPID_BLINK = 1 << 3;
		/// Reversed foreground and background colors
		const REVERSED = 1 << 4;
		/// Hidden text
		const HIDDEN = 1 << 5;
		/// `text-decoration-line: underline`
		const UNDERLINE = 1 << 6;
		/// `text-decoration-line: overline`
		const OVERLINE = 1 << 7;
		/// `text-decoration-line: line-through`
		const LINE_THROUGH = 1 << 8;
		/// `text-decoration-style: wavy` (combine with UNDERLINE/OVERLINE)
		const WAVY = 1 << 9;
		/// `text-decoration-style: double` (combine with UNDERLINE/OVERLINE)
		const DOUBLE = 1 << 10;
		/// `text-decoration-style: dashed` (combine with UNDERLINE/OVERLINE)
		const DASH = 1 << 11;
	}
}
