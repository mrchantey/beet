//! Cross-platform visual styling for terminal and char-cell renderers.
//!
//! [`VisualStyle`] is the single styling type used by the char-cell renderer
//! and the ANSI escape bridge. Colours are always [bevy `Color`](Color)s; a
//! foreground alpha of `<= 0.5` applies the terminal `dim` attribute. This
//! module also adds the beet_ui-specific [`AsCssValue`]/[`AsCssValues`]
//! implementations for the style sub-types.
use crate::style::AsCssValue;
use crate::style::AsCssValues;
use crate::style::CssKey;
use crate::style::CssValue;
use beet_core::prelude::*;
use bitflags::bitflags;
use std::io;
use std::io::Write;

/// The default [`VisualStyle`], usable in `const` contexts.
pub static VISUAL_STYLE_DEFAULT: VisualStyle = VisualStyle::DEFAULT;

/// Visual styling for a cell or run of text.
#[derive(Debug, Default, Clone, PartialEq, SetWith, Component)]
pub struct VisualStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attribute
	#[set_with(unwrap_option, into)]
	pub foreground: Option<Color>,
	/// Background fill colour.
	#[set_with(unwrap_option, into)]
	pub background: Option<Color>,
	/// Color of underlines, overlines etc
	pub decoration_color: Option<Color>,
	/// Which decoration lines (underline, overline, strikethrough) are drawn.
	pub decoration_line: DecorationLine,
	/// Visual style of the decoration lines.
	pub decoration_style: DecorationStyle,
	/// Text attribute flags (bold, italic, blink, etc).
	pub text_style: TextStyle,
	/// Horizontal alignment of text within its content area.
	pub text_align: TextAlign,
}

impl VisualStyle {
	/// The default style, applying no attributes.
	pub const DEFAULT: Self = Self {
		foreground: None,
		background: None,
		decoration_color: None,
		decoration_line: DecorationLine::DEFAULT,
		decoration_style: DecorationStyle::Solid,
		text_style: TextStyle::empty(),
		text_align: TextAlign::Left,
	};

	/// An empty style that applies no attributes.
	pub fn new() -> Self { Self::DEFAULT }

	/// Set the foreground colour.
	pub fn fg(mut self, color: impl Into<Color>) -> Self {
		self.foreground = Some(color.into());
		self
	}

	/// Set the background colour.
	pub fn on(mut self, color: impl Into<Color>) -> Self {
		self.background = Some(color.into());
		self
	}

	/// Enable the bold attribute.
	pub fn bold(mut self) -> Self {
		self.text_style |= TextStyle::BOLD;
		self
	}

	/// Enable the italic attribute.
	pub fn italic(mut self) -> Self {
		self.text_style |= TextStyle::ITALIC;
		self
	}

	/// Dim the style by lowering the foreground alpha to `0.5`, which the
	/// ANSI bridge renders as the terminal `faint` attribute. When no
	/// foreground is set a neutral grey is used.
	pub fn dimmed(mut self) -> Self {
		self.foreground = Some(match self.foreground {
			Some(color) => color.with_alpha(0.5),
			None => Color::srgba(0.6, 0.6, 0.6, 0.5),
		});
		self
	}

	/// Whether the foreground colour is set with an alpha of `0.5` or less.
	pub fn dim_foreground(&self) -> bool {
		match self.foreground {
			Some(color) => color.alpha() <= 0.5,
			None => false,
		}
	}

	/// Whether this style would emit no escape codes.
	pub fn is_plain(&self) -> bool {
		self.foreground.is_none()
			&& self.background.is_none()
			&& self.decoration_color.is_none()
			&& self.decoration_line == DecorationLine::DEFAULT
			&& self.text_style.is_empty()
	}

	/// Write only the SGR changes needed to transition from `prev` to `self`.
	///
	/// If `prev` is `None`, the full style is written unconditionally.
	/// When text attributes or dim-state change, a `RESET` is emitted first and
	/// all colours and attributes are re-applied from scratch to avoid partial
	/// state conflicts (eg `\x1b[22m` clears both bold and faint simultaneously).
	pub fn write_style(
		&self,
		w: &mut impl Write,
		prev: Option<&VisualStyle>,
	) -> io::Result<()> {
		let is_dim = self.dim_foreground();
		let prev_dim = prev.map_or(false, |p| p.dim_foreground());
		let text_changed = prev.map_or(true, |p| {
			p.text_style != self.text_style
				|| p.decoration_line != self.decoration_line
		}) || is_dim != prev_dim;

		// When text attributes changed, reset everything and re-apply from
		// scratch. This avoids the complexity of SGR-off codes interacting
		// (eg BOLD_FAINT_OFF).
		let effective_prev: Option<&VisualStyle> = if text_changed {
			w.write_all(escape::RESET.as_bytes())?;
			None // after RESET, all attributes are cleared
		} else {
			prev
		};

		if self.foreground != effective_prev.and_then(|p| p.foreground) {
			match self.foreground {
				Some(color) => escape::foreground(w, color)?,
				None => w.write_all(escape::RESET_FG.as_bytes())?,
			}
		}
		if self.background != effective_prev.and_then(|p| p.background) {
			match self.background {
				Some(color) => escape::background(w, color)?,
				None => w.write_all(escape::RESET_BG.as_bytes())?,
			}
		}

		// Text attributes and decoration lines — only written when text_changed.
		// RESET already cleared all attributes, so we re-apply only what is active.
		if text_changed {
			if is_dim {
				w.write_all(escape::FAINT.as_bytes())?;
			}
			let ts = self.text_style;
			if ts.contains(TextStyle::BOLD) {
				w.write_all(escape::BOLD.as_bytes())?;
			}
			if ts.contains(TextStyle::ITALIC) {
				w.write_all(escape::ITALIC.as_bytes())?;
			}
			if ts.contains(TextStyle::BLINK) {
				w.write_all(escape::BLINK.as_bytes())?;
			}
			if ts.contains(TextStyle::RAPID_BLINK) {
				w.write_all(escape::RAPID_BLINK.as_bytes())?;
			}
			if ts.contains(TextStyle::REVERSED) {
				w.write_all(escape::INVERT.as_bytes())?;
			}
			if ts.contains(TextStyle::HIDDEN) {
				w.write_all(escape::HIDDEN.as_bytes())?;
			}
			// Decoration lines: re-apply after reset; absence is the cleared state.
			let dl = self.decoration_line;
			if dl.underline {
				w.write_all(escape::UNDERLINE.as_bytes())?;
			}
			if dl.overline {
				w.write_all(escape::OVERLINE.as_bytes())?;
			}
			if dl.line_through {
				w.write_all(escape::CROSSED_OUT.as_bytes())?;
			}
		}

		Ok(())
	}
}

/// Text alignment within a cell's content area.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TextAlign {
	/// Align to the left edge.
	#[default]
	Left,
	/// Center within the content area.
	Center,
	/// Align to the right edge.
	Right,
}

/// Which decoration lines a run of text carries.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DecorationLine {
	/// Draw a line below the text.
	pub underline: bool,
	/// Draw a line above the text.
	pub overline: bool,
	/// Draw a line through the text.
	pub line_through: bool,
}

impl DecorationLine {
	/// No decoration lines.
	pub const DEFAULT: Self = Self {
		underline: false,
		overline: false,
		line_through: false,
	};

	/// Underline only.
	pub fn underline() -> Self {
		Self {
			underline: true,
			overline: false,
			line_through: false,
		}
	}
	/// Overline only.
	pub fn overline() -> Self {
		Self {
			underline: false,
			overline: true,
			line_through: false,
		}
	}
	/// Strikethrough only.
	pub fn line_through() -> Self {
		Self {
			line_through: true,
			underline: false,
			overline: false,
		}
	}
}

/// The style of a decoration line. Terminal support is limited, so all
/// variants render as a plain line.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DecorationStyle {
	/// A single solid line.
	#[default]
	Solid,
	/// A wavy line.
	Wavy,
	/// A double line.
	Double,
	/// A dashed line.
	Dash,
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

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn write_style_transitions_underline_off() {
		// Transitioning from underline=true to underline=false must emit RESET
		// and must NOT re-write UNDERLINE.
		let style_with = VisualStyle {
			decoration_line: DecorationLine::underline(),
			..default()
		};
		let style_without = VisualStyle::default();
		let mut buf: Vec<u8> = Vec::new();
		style_without.write_style(&mut buf, Some(&style_with)).unwrap();
		let out = String::from_utf8(buf).unwrap();
		out.as_str().xpect_contains("\x1b[0m");
		out.xnot().xpect_contains("\x1b[4m");
	}
}
