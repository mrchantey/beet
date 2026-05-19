//! Cross-platform visual styling for terminal and char-cell renderers.
//!
//! [`VisualStyle`] is the single styling type used by the ANSI escape bridge
//! in [`terminal::escape`](crate::terminal::escape) and the char-cell renderer.
//! Colours are always [bevy `Color`](Color)s; a foreground alpha of `<= 0.5`
//! applies the terminal `dim` attribute.
use crate::prelude::*;
use bitflags::bitflags;

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
