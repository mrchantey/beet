//! Unix terminal utilities
use std::io::Write;
use std::io::stdout;

/// Shows the terminal cursor.
///
/// ```
/// # use beet_core::prelude::*;
/// terminal_ext::show_cursor();
/// ```
pub fn show_cursor() {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[?25h").unwrap();
}

/// Hides the terminal cursor.
///
/// ```
/// # use beet_core::prelude::*;
/// terminal_ext::hide_cursor();
/// ```
pub fn hide_cursor() {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[?25l").unwrap();
}

/// Resets the cursor to the home position (0, 0).
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::reset_cursor();
/// ```
pub fn reset_cursor() { move_to(0, 0).ok(); }

/// Clears the terminal screen and moves the cursor to the home position.
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::clear().unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if the write or flush operations fail.
pub fn clear() -> std::io::Result<()> {
	let mut stdout = stdout();
	stdout.write_all(b"\x1B[2J")?;
	stdout.write_all(b"\x1B[H")?;
	stdout.flush()?;
	Ok(())
}


/// Moves the cursor to the specified position (x, y).
///
/// # Arguments
///
/// * `x` - The column position (0-indexed)
/// * `y` - The row position (0-indexed)
///
/// # Examples
///
/// ```no_run
/// # use beet_core::prelude::*;
/// terminal_ext::move_to(10, 5).unwrap();
/// ```
///
/// # Errors
///
/// Returns an error if the write or flush operations fail.
pub fn move_to(x: u16, y: u16) -> std::io::Result<()> {
	let mut stdout = stdout();
	stdout.write_all(format!("\x1B[{};{}H", y + 1, x + 1).as_bytes())?;
	stdout.flush()?;
	Ok(())
}
