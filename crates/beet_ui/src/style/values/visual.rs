use beet_core::prelude::*;
use bitflags::bitflags;

use crate::style::AsCssValue;
use crate::style::AsCssValues;
use crate::style::CssKey;
use crate::style::CssValue;

pub static VISUAL_STYLE_DEFAULT: VisualStyle = VisualStyle::DEFAULT;


/// Visual styling for a cell.
#[derive(Debug, Default, Clone, PartialEq, SetWith, Component)]
pub struct VisualStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attribute
	#[set_with(unwrap_option, into)]
	pub foreground: Option<Color>,
	#[set_with(unwrap_option, into)]
	pub background: Option<Color>,
	/// Color of underlines, overlines etc
	pub decoration_color: Option<Color>,
	pub decoration_line: DecorationLine,
	pub decoration_style: DecorationStyle,
	pub text_style: TextStyle,
	pub text_align: TextAlign,
}

impl VisualStyle {
	pub const DEFAULT: Self = Self {
		foreground: None,
		background: None,
		decoration_color: None,
		decoration_line: DecorationLine::DEFAULT,
		decoration_style: DecorationStyle::Solid,
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
	pub underline: bool,
	pub overline: bool,
	pub line_through: bool,
}

impl DecorationLine {
	pub const DEFAULT: Self = Self {
		underline: false,
		overline: false,
		line_through: false,
	};

	pub fn underline() -> Self {
		Self {
			underline: true,
			overline: false,
			line_through: false,
		}
	}
	pub fn overline() -> Self {
		Self {
			underline: false,
			overline: true,
			line_through: false,
		}
	}
	pub fn line_through() -> Self {
		Self {
			line_through: true,
			underline: false,
			overline: false,
		}
	}
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
	}
}


impl AsCssValues for TextStyle {
	fn suffixes() -> Vec<CssKey> {
		vec![
			CssKey::static_property("bold"),
			CssKey::static_property("italic"),
			CssKey::static_property("blink"),
			CssKey::static_property("rapidBlink"),
			CssKey::static_property("reversed"),
			CssKey::static_property("hidden"),
		]
	}

	fn as_css_values(&self) -> Result<Vec<CssValue>> {
		// TODO this is very incorrect,
		// the type probs needs to be reworked, split
		// based on css property mappings
		let mut values = Vec::new();
		if self.contains(Self::BOLD) {
			values.push(CssValue::expression("bold"));
		}
		if self.contains(Self::ITALIC) {
			values.push(CssValue::expression("italic"));
		}
		if self.contains(Self::BLINK) {
			values.push(CssValue::expression("blink"));
		}
		if self.contains(Self::RAPID_BLINK) {
			values.push(CssValue::expression("rapid-blink"));
		}
		if self.contains(Self::REVERSED) {
			values.push(CssValue::expression("reversed"));
		}
		if self.contains(Self::HIDDEN) {
			values.push(CssValue::expression("hidden"));
		}
		if values.is_empty() {
			values.push(CssValue::expression("normal"));
		}
		values.xok()
	}
}
