//! Unix terminal utilities
#[allow(unused)]
use crate::prelude::*;
use std::io::Write;
use std::io::stdout;

/// Adds this handler to both the panic and ctrl+c hooks
pub fn on_force_exit(
	func: impl 'static + Send + Sync + Clone + Fn(),
) -> Result {
	#[cfg(all(not(target_arch = "wasm32"), feature = "ctrlc"))]
	{
		let func2 = func.clone();
		ctrlc::set_handler(move || {
			func2();
			std::process::exit(0);
		})?;
	}
	// update_hook when stablizes
	let prev = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		func();
		prev(info);
	}));
	Ok(())
}


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




/// Returns the terminal size, defaulting to 80,24 if it could not be determined.
pub fn size() -> UVec2 {
	let default_size = UVec2::new(80, 24);
	cfg_if! {
		if #[cfg(all(not(target_arch = "wasm32"), feature = "crossterm"))]{
			crossterm::terminal::size()
				.map(|(cols,rows)|UVec2::new(cols as u32, rows as u32))
				.unwrap_or(default_size)
		}else {
			default_size
		}
	}
}
