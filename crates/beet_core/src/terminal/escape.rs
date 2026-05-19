//! Raw ANSI/VT100 terminal escape sequences.
//!
//! Raw CSI and SGR sequences for controlling terminal display and input,
//! plus [`foreground_term`]/[`background_term`], which write the 16-colour,
//! 256-colour and 24-bit sequences for a [`TermColor`](super::term_style::TermColor)
//! used by [`TermStyle`](super::term_style::TermStyle).
use crate::prelude::*;
use crate::terminal::term_style::TermColor;
use std::io;
use std::io::Write;

// ── Screen ────────────────────────────────────────────────────────────────────

/// Switch to the alternate screen buffer.
pub const ENTER_ALT_SCREEN: &str = "\x1b[?1049h";
/// Restore the main screen buffer.
pub const LEAVE_ALT_SCREEN: &str = "\x1b[?1049l";

// ── Cursor ────────────────────────────────────────────────────────────────────

/// Hide the text cursor.
pub const HIDE_CURSOR: &str = "\x1b[?25l";
/// Show the text cursor.
pub const SHOW_CURSOR: &str = "\x1b[?25h";
/// Move cursor to home position (1,1).
pub const CURSOR_HOME: &str = "\x1b[H";
/// Save the cursor position.
pub const CURSOR_SAVE: &str = "\x1b[s";
/// Restore the cursor position.
pub const CURSOR_RESTORE: &str = "\x1b[u";

// ── Mouse ─────────────────────────────────────────────────────────────────────

/// Enable mouse tracking: click, button-motion, any-motion, and SGR encoding.
///
/// Enables in order: X10 (`?1000h`), button-event (`?1002h`),
/// any-motion (`?1003h`), URXVT decimal (`?1015h`), SGR (`?1006h`).
pub const ENTER_MOUSE: &str =
	"\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1015h\x1b[?1006h";
/// Disable mouse tracking (reverses [`ENTER_MOUSE`]).
pub const LEAVE_MOUSE: &str =
	"\x1b[?1006l\x1b[?1015l\x1b[?1003l\x1b[?1002l\x1b[?1000l";

// ── Bracketed Paste ───────────────────────────────────────────────────────────

/// Enable bracketed paste mode — pastes arrive wrapped in `CSI 200~` / `CSI 201~`.
pub const ENTER_BRACKETED_PASTE: &str = "\x1b[?2004h";
/// Disable bracketed paste mode.
pub const LEAVE_BRACKETED_PASTE: &str = "\x1b[?2004l";

// ── Erase ─────────────────────────────────────────────────────────────────────

/// Erase the entire screen.
pub const ERASE_ALL: &str = "\x1b[2J";
/// Erase from cursor to end of line.
pub const ERASE_LINE: &str = "\x1b[K";

// ── SGR attributes ────────────────────────────────────────────────────────────

/// Reset all SGR attributes.
pub const RESET: &str = "\x1b[0m";
/// Reset foreground colour to default.
pub const RESET_FG: &str = "\x1b[39m";
/// Reset background colour to default.
pub const RESET_BG: &str = "\x1b[49m";
/// Bold text.
pub const BOLD: &str = "\x1b[1m";
/// Dim/faint text.
pub const FAINT: &str = "\x1b[2m";
/// Italic text.
pub const ITALIC: &str = "\x1b[3m";
/// Underlined text.
pub const UNDERLINE: &str = "\x1b[4m";
/// Slow-blink text.
pub const BLINK: &str = "\x1b[5m";
/// Rapid-blink text.
pub const RAPID_BLINK: &str = "\x1b[6m";
/// Swap foreground and background colours.
pub const INVERT: &str = "\x1b[7m";
/// Hidden/invisible text.
pub const HIDDEN: &str = "\x1b[8m";
/// Strikethrough text.
pub const CROSSED_OUT: &str = "\x1b[9m";
/// Overlined text.
pub const OVERLINE: &str = "\x1b[53m";

// ── SGR off ───────────────────────────────────────────────────────────────────

/// Turn off bold and faint/dim (both use the same reset code).
pub const BOLD_FAINT_OFF: &str = "\x1b[22m";
/// Turn off italic.
pub const ITALIC_OFF: &str = "\x1b[23m";
/// Turn off underline.
pub const UNDERLINE_OFF: &str = "\x1b[24m";
/// Turn off blink (slow and rapid share a reset code).
pub const BLINK_OFF: &str = "\x1b[25m";
/// Turn off invert/reverse.
pub const INVERT_OFF: &str = "\x1b[27m";
/// Turn off hidden.
pub const HIDDEN_OFF: &str = "\x1b[28m";
/// Turn off strikethrough.
pub const CROSSED_OUT_OFF: &str = "\x1b[29m";
/// Turn off overline.
pub const OVERLINE_OFF: &str = "\x1b[55m";

// ── Positioning ───────────────────────────────────────────────────────────────

/// Write a cursor-goto sequence for a 0-indexed [`UVec2`] position.
pub fn cursor_goto(w: &mut impl Write, pos: UVec2) -> io::Result<()> {
	// ANSI format: row ; col (1-indexed)
	write!(w, "\x1b[{};{}H", pos.y + 1, pos.x + 1)
}

/// Write cursor up by `n` rows.
pub fn cursor_up(w: &mut impl Write, n: u32) -> io::Result<()> {
	write!(w, "\x1b[{n}A")
}

/// Write cursor down by `n` rows.
pub fn cursor_down(w: &mut impl Write, n: u32) -> io::Result<()> {
	write!(w, "\x1b[{n}B")
}

/// Write cursor right by `n` columns.
pub fn cursor_right(w: &mut impl Write, n: u32) -> io::Result<()> {
	write!(w, "\x1b[{n}C")
}

/// Write cursor left by `n` columns.
pub fn cursor_left(w: &mut impl Write, n: u32) -> io::Result<()> {
	write!(w, "\x1b[{n}D")
}

// ── 24-bit colour ─────────────────────────────────────────────────────────────

/// Write a 24-bit foreground colour from a bevy [`Color`].
pub fn foreground(w: &mut impl Write, color: Color) -> io::Result<()> {
	let c = color.to_srgba().to_u8_array();
	write!(w, "\x1b[38;2;{};{};{}m", c[0], c[1], c[2])
}

/// Write a 24-bit background colour from a bevy [`Color`].
pub fn background(w: &mut impl Write, color: Color) -> io::Result<()> {
	let c = color.to_srgba().to_u8_array();
	write!(w, "\x1b[48;2;{};{};{}m", c[0], c[1], c[2])
}

// ── Terminal palette colour ───────────────────────────────────────────────────

/// Write a foreground [`TermColor`] using the 16-colour, 256-colour or 24-bit
/// sequence as appropriate.
pub fn foreground_term(
	w: &mut impl Write,
	color: TermColor,
) -> io::Result<()> {
	match color.palette_index() {
		Some(index) if index < 8 => write!(w, "\x1b[{}m", 30 + index),
		Some(index) => write!(w, "\x1b[{}m", 90 + index - 8),
		None => match color {
			TermColor::Fixed(n) => write!(w, "\x1b[38;5;{n}m"),
			TermColor::Rgb(r, g, b) => write!(w, "\x1b[38;2;{r};{g};{b}m"),
			_ => unreachable!("named colours return a palette index"),
		},
	}
}

/// Write a background [`TermColor`] using the 16-colour, 256-colour or 24-bit
/// sequence as appropriate.
pub fn background_term(
	w: &mut impl Write,
	color: TermColor,
) -> io::Result<()> {
	match color.palette_index() {
		Some(index) if index < 8 => write!(w, "\x1b[{}m", 40 + index),
		Some(index) => write!(w, "\x1b[{}m", 100 + index - 8),
		None => match color {
			TermColor::Fixed(n) => write!(w, "\x1b[48;5;{n}m"),
			TermColor::Rgb(r, g, b) => write!(w, "\x1b[48;2;{r};{g};{b}m"),
			_ => unreachable!("named colours return a palette index"),
		},
	}
}

