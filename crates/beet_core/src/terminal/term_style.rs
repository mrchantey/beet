//! Real terminal colours and styling.
//!
//! [`TermStyle`] is the only way to apply genuine terminal colours: a
//! [`TermColor`] resolves to the terminal's own 16-colour palette (or a
//! 256-colour / 24-bit sequence) rather than an approximated bevy `Color`.
//! Build a style, then [`paint`](TermStyle::paint) a value to wrap it in the
//! matching escape sequences.
//!
//! ```
//! use beet_core::prelude::*;
//! let message = TermStyle::green().bold().paint("Success!");
//! println!("{message}");
//! ```
use crate::prelude::escape;
use crate::prelude::escape::RESET;
use std::fmt::Display;

/// A terminal colour, resolved against the terminal's own palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermColor {
	/// Palette colour 0.
	Black,
	/// Palette colour 1.
	Red,
	/// Palette colour 2.
	Green,
	/// Palette colour 3.
	Yellow,
	/// Palette colour 4.
	Blue,
	/// Palette colour 5.
	Magenta,
	/// Palette colour 6.
	Cyan,
	/// Palette colour 7.
	White,
	/// Bright palette colour 8.
	BrightBlack,
	/// Bright palette colour 9.
	BrightRed,
	/// Bright palette colour 10.
	BrightGreen,
	/// Bright palette colour 11.
	BrightYellow,
	/// Bright palette colour 12.
	BrightBlue,
	/// Bright palette colour 13.
	BrightMagenta,
	/// Bright palette colour 14.
	BrightCyan,
	/// Bright palette colour 15.
	BrightWhite,
	/// A 256-colour palette index.
	Fixed(u8),
	/// A 24-bit true colour.
	Rgb(u8, u8, u8),
}

impl TermColor {
	/// The `0..16` palette index for a named colour, or `None` for
	/// [`Fixed`](TermColor::Fixed) and [`Rgb`](TermColor::Rgb).
	pub fn palette_index(&self) -> Option<u8> {
		use TermColor::*;
		let index = match self {
			Black => 0,
			Red => 1,
			Green => 2,
			Yellow => 3,
			Blue => 4,
			Magenta => 5,
			Cyan => 6,
			White => 7,
			BrightBlack => 8,
			BrightRed => 9,
			BrightGreen => 10,
			BrightYellow => 11,
			BrightBlue => 12,
			BrightMagenta => 13,
			BrightCyan => 14,
			BrightWhite => 15,
			Fixed(_) | Rgb(..) => return None,
		};
		Some(index)
	}
}

/// A real-terminal text style: palette colours plus the common attributes.
///
/// A default [`TermStyle`] applies nothing, so [`paint`](TermStyle::paint)
/// returns the value unchanged. This makes the no-colour case a plain default
/// rather than a global toggle.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TermStyle {
	/// Foreground colour.
	pub foreground: Option<TermColor>,
	/// Background colour.
	pub background: Option<TermColor>,
	/// Bold attribute.
	pub bold: bool,
	/// Dim/faint attribute.
	pub dimmed: bool,
	/// Italic attribute.
	pub italic: bool,
	/// Underline attribute.
	pub underline: bool,
}

impl TermStyle {
	/// An empty style that applies no attributes.
	pub fn new() -> Self { Self::default() }

	/// Set the foreground colour.
	pub fn fg(mut self, color: TermColor) -> Self {
		self.foreground = Some(color);
		self
	}

	/// Set the background colour.
	pub fn on(mut self, color: TermColor) -> Self {
		self.background = Some(color);
		self
	}

	/// Enable the bold attribute.
	pub fn bold(mut self) -> Self {
		self.bold = true;
		self
	}

	/// Enable the dim/faint attribute.
	pub fn dimmed(mut self) -> Self {
		self.dimmed = true;
		self
	}

	/// Enable the italic attribute.
	pub fn italic(mut self) -> Self {
		self.italic = true;
		self
	}

	/// Enable the underline attribute.
	pub fn underline(mut self) -> Self {
		self.underline = true;
		self
	}

	/// A red foreground style.
	pub fn red() -> Self { Self::new().fg(TermColor::Red) }
	/// A green foreground style.
	pub fn green() -> Self { Self::new().fg(TermColor::Green) }
	/// A cyan foreground style.
	pub fn cyan() -> Self { Self::new().fg(TermColor::Cyan) }
	/// A yellow foreground style.
	pub fn yellow() -> Self { Self::new().fg(TermColor::Yellow) }
	/// A blue foreground style.
	pub fn blue() -> Self { Self::new().fg(TermColor::Blue) }

	/// Whether this style would emit no escape codes.
	pub fn is_plain(&self) -> bool {
		self.foreground.is_none()
			&& self.background.is_none()
			&& !self.bold
			&& !self.dimmed
			&& !self.italic
			&& !self.underline
	}

	/// Returns this style, or the plain default style when `color` is `false`.
	///
	/// Used in place of a global colour toggle: callers pass their own
	/// colour-enabled flag (eg `!config.no_color`).
	pub fn or_plain(self, color: bool) -> Self {
		if color { self } else { Self::default() }
	}

	/// The opening SGR sequence for this style, or empty when
	/// [`is_plain`](TermStyle::is_plain).
	pub fn prefix(&self) -> String {
		if self.is_plain() {
			return String::new();
		}
		let mut buf: Vec<u8> = Vec::new();
		// writing to a Vec cannot fail
		if let Some(color) = self.foreground {
			escape::foreground_term(&mut buf, color).ok();
		}
		if let Some(color) = self.background {
			escape::background_term(&mut buf, color).ok();
		}
		if self.bold {
			buf.extend_from_slice(escape::BOLD.as_bytes());
		}
		if self.dimmed {
			buf.extend_from_slice(escape::FAINT.as_bytes());
		}
		if self.italic {
			buf.extend_from_slice(escape::ITALIC.as_bytes());
		}
		if self.underline {
			buf.extend_from_slice(escape::UNDERLINE.as_bytes());
		}
		String::from_utf8_lossy(&buf).into_owned()
	}

	/// Wrap `val` in this style's prefix and a trailing [`RESET`].
	///
	/// Plain styles return the value unchanged.
	pub fn paint(&self, val: impl Display) -> String {
		if self.is_plain() {
			return val.to_string();
		}
		format!("{}{val}{RESET}", self.prefix())
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;

	#[crate::test]
	fn plain_style_emits_nothing() {
		TermStyle::new().is_plain().xpect_true();
		TermStyle::new().prefix().xpect_eq("");
		TermStyle::new().paint("hi").xpect_eq("hi");
	}

	#[crate::test]
	fn or_plain_strips_when_disabled() {
		TermStyle::red().or_plain(false).paint("hi").xpect_eq("hi");
		TermStyle::red()
			.or_plain(true)
			.paint("hi")
			.xpect_contains("\x1b[31m");
	}

	#[crate::test]
	fn paint_wraps_named_and_resets() {
		TermStyle::new()
			.fg(TermColor::Red)
			.on(TermColor::Green)
			.bold()
			.paint("hi")
			.xpect_eq("\x1b[31m\x1b[42m\x1b[1mhi\x1b[0m");
	}

	#[crate::test]
	fn fixed_and_rgb_sequences() {
		TermStyle::new()
			.fg(TermColor::Fixed(22))
			.paint("x")
			.xpect_eq("\x1b[38;5;22mx\x1b[0m");
		TermStyle::new()
			.on(TermColor::Rgb(1, 2, 3))
			.paint("x")
			.xpect_eq("\x1b[48;2;1;2;3mx\x1b[0m");
	}
}
