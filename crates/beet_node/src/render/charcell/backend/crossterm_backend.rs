use crate::prelude::*;
use beet_core::prelude::*;
use std::io::Write;

/// Terminal backend that writes ANSI escape sequences to any [`Write`] target.
pub struct CrosstermBackend<W: Write> {
	writer: W,
}

impl<W: Write> CrosstermBackend<W> {
	pub fn new(writer: W) -> Self { Self { writer } }

	pub fn writer(&self) -> &W { &self.writer }

	pub fn writer_mut(&mut self) -> &mut W { &mut self.writer }
}

impl<W: Write> Backend for CrosstermBackend<W> {
	fn hide_cursor(&mut self) -> Result {
		write!(self.writer, "\x1B[?25l")?;
		Ok(())
	}

	fn show_cursor(&mut self) -> Result {
		write!(self.writer, "\x1B[?25h")?;
		Ok(())
	}

	/// Always returns `UVec2::ZERO` — cursor position cannot be queried without raw mode.
	fn get_cursor(&mut self) -> Result<UVec2> { Ok(UVec2::ZERO) }

	fn set_cursor(&mut self, pos: UVec2) -> Result {
		// ANSI move: ESC[row;colH (1-indexed)
		write!(self.writer, "\x1B[{};{}H", pos.y + 1, pos.x + 1)?;
		Ok(())
	}

	fn clear(&mut self) -> Result {
		write!(self.writer, "\x1B[2J\x1B[H")?;
		Ok(())
	}

	fn window_size(&mut self) -> Result<WindowSize> {
		let chars = terminal_ext::size().unwrap_or(UVec2::new(80, 24));
		Ok(WindowSize {
			chars,
			pixels: UVec2::ZERO,
		})
	}

	fn draw(&mut self, buffer: &Buffer) -> Result {
		// Move to home position then write the full rendered buffer
		write!(self.writer, "\x1B[H{}", buffer.render())?;
		Ok(())
	}

	fn flush(&mut self) -> Result {
		self.writer.flush()?;
		Ok(())
	}
}
