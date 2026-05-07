//! ANSI/VT100 terminal escape sequences.
//!
//! Raw CSI and SGR sequences for controlling terminal display and input.
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

// ── Erase ─────────────────────────────────────────────────────────────────────

/// Erase the entire screen.
pub const ERASE_ALL: &str = "\x1b[2J";

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

// ── Positioning / Color ───────────────────────────────────────────────────────

/// Write a cursor-goto sequence for 0-indexed `(col, row)`.
pub fn write_goto(w: &mut impl Write, col: u32, row: u32) -> io::Result<()> {
	write!(w, "\x1b[{};{}H", row + 1, col + 1)
}

/// Write a 24-bit foreground colour.
pub fn write_fg(w: &mut impl Write, r: u8, g: u8, b: u8) -> io::Result<()> {
	write!(w, "\x1b[38;2;{r};{g};{b}m")
}

/// Write a 24-bit background colour.
pub fn write_bg(w: &mut impl Write, r: u8, g: u8, b: u8) -> io::Result<()> {
	write!(w, "\x1b[48;2;{r};{g};{b}m")
}
