//! beet_ui's [`VisualStyle`] → ANSI bridge.
//!
//! Re-exports the raw escape sequences from [`beet_core`] and adds
//! [`write_style`], which translates a [`VisualStyle`] into SGR codes.
pub use beet_core::prelude::escape::*;
use crate::style::*;
use beet_core::prelude::Color as RgbColor;
use beet_core::prelude::*;
use std::io;
use std::io::Write;

// ── 24-bit colour ─────────────────────────────────────────────────────────────

/// Write a 24-bit foreground colour from a bevy [`Color`].
pub fn foreground(w: &mut impl Write, color: RgbColor) -> io::Result<()> {
	let c = color.to_srgba_u8();
	write!(w, "\x1b[38;2;{};{};{}m", c.red, c.green, c.blue)
}

/// Write a 24-bit background colour from a bevy [`Color`].
pub fn background(w: &mut impl Write, color: RgbColor) -> io::Result<()> {
	let c = color.to_srgba_u8();
	write!(w, "\x1b[48;2;{};{};{}m", c.red, c.green, c.blue)
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Write only the SGR changes needed to transition from `prev` style to `style`.
///
/// If `prev` is `None`, the full style is written unconditionally.
/// When text attributes or dim-state change, a `RESET` is emitted first and
/// all colours and attributes are re-applied from scratch to avoid partial
/// state conflicts (eg `\x1b[22m` clears both bold and faint simultaneously).
pub fn write_style(
	w: &mut impl Write,
	style: &VisualStyle,
	prev: Option<&VisualStyle>,
) -> io::Result<()> {
	let is_dim = style.dim_foreground();
	let prev_dim = prev.map_or(false, |p| p.dim_foreground());
	let text_changed = prev.map_or(true, |p| {
		p.text_style != style.text_style
			|| p.decoration_line != style.decoration_line
	}) || is_dim != prev_dim;

	// When text attributes changed, reset everything and re-apply from scratch.
	// This avoids the complexity of SGR-off codes interacting (eg BOLD_FAINT_OFF).
	let effective_prev: Option<&VisualStyle> = if text_changed {
		w.write_all(RESET.as_bytes())?;
		None // after RESET, all attributes are cleared
	} else {
		prev
	};

	if style.foreground != effective_prev.and_then(|p| p.foreground) {
		match style.foreground {
			Some(color) => foreground(w, color)?,
			None => w.write_all(RESET_FG.as_bytes())?,
		}
	}
	if style.background != effective_prev.and_then(|p| p.background) {
		match style.background {
			Some(color) => background(w, color)?,
			None => w.write_all(RESET_BG.as_bytes())?,
		}
	}

	match style.decoration_style {
		DecorationStyle::Solid => {}
		// These styles have limited terminal support;
		// rendered as plain underline when decoration_line.underline is set.
		DecorationStyle::Double
		| DecorationStyle::Wavy
		| DecorationStyle::Dash => {}
	}

	// Text attributes and decoration lines — only written when text_changed.
	// RESET already cleared all attributes, so we re-apply only what is active.
	if text_changed {
		if is_dim {
			w.write_all(FAINT.as_bytes())?;
		}
		let ts = style.text_style;
		if ts.contains(TextStyle::BOLD) {
			w.write_all(BOLD.as_bytes())?;
		}
		if ts.contains(TextStyle::ITALIC) {
			w.write_all(ITALIC.as_bytes())?;
		}
		if ts.contains(TextStyle::BLINK) {
			w.write_all(BLINK.as_bytes())?;
		}
		if ts.contains(TextStyle::RAPID_BLINK) {
			w.write_all(RAPID_BLINK.as_bytes())?;
		}
		if ts.contains(TextStyle::REVERSED) {
			w.write_all(INVERT.as_bytes())?;
		}
		if ts.contains(TextStyle::HIDDEN) {
			w.write_all(HIDDEN.as_bytes())?;
		}
		// Decoration lines: re-apply after reset; absence is the cleared state.
		let dl = style.decoration_line;
		if dl.underline {
			w.write_all(UNDERLINE.as_bytes())?;
		}
		if dl.overline {
			w.write_all(OVERLINE.as_bytes())?;
		}
		if dl.line_through {
			w.write_all(CROSSED_OUT.as_bytes())?;
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	/// Render a bundle to a 40×5 buffer and return the trimmed plain string.
	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(40, 5), bundle)
			.trim_lines()
	}

	fn bordered() -> BoxStyle {
		BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)))
	}

	#[beet_core::test]
	fn underline_does_not_bleed_into_border() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Hello" },
			bordered(),
			VisualStyle {
				decoration_line: DecorationLine::underline(),
				..default()
			}
		)]));
		out.as_str().xpect_contains("┌");
		out.xpect_contains("Hello");
	}

	#[beet_core::test]
	fn strike_does_not_bleed_into_border() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Hi" },
			bordered(),
			VisualStyle {
				decoration_line: DecorationLine::line_through(),
				..default()
			}
		)]));
		out.as_str().xpect_contains("┌");
		out.xpect_contains("Hi");
	}

	#[beet_core::test]
	fn italic_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Italic" },
			VisualStyle {
				text_style: TextStyle::ITALIC,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[3m");
	}

	#[beet_core::test]
	fn bold_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Bold" },
			VisualStyle {
				text_style: TextStyle::BOLD,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[1m");
	}

	#[beet_core::test]
	fn blink_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Blink" },
			VisualStyle {
				text_style: TextStyle::BLINK,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[5m");
	}

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
		write_style(&mut buf, &style_without, Some(&style_with)).unwrap();
		let out = String::from_utf8(buf).unwrap();
		out.as_str().xpect_contains("\x1b[0m");
		out.xnot().xpect_contains("\x1b[4m");
	}
}
