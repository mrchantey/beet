use crate::prelude::*;
use beet_core::prelude::*;

/// An in-memory backend for use in tests and headless environments.
#[derive(Get)]
pub struct TestBackend {
	buffer: Buffer,
	cursor_position: UVec2,
	cursor_hidden: bool,
}

impl TestBackend {
	/// Create a new backend with a blank buffer of the given size.
	pub fn new(size: UVec2) -> Self {
		Self {
			buffer: Buffer::new(size),
			cursor_position: UVec2::ZERO,
			cursor_hidden: false,
		}
	}
	pub fn size(&self) -> UVec2 { self.buffer.size() }
}

impl Backend for TestBackend {
	fn hide_cursor(&mut self) -> Result {
		self.cursor_hidden = true;
		Ok(())
	}

	fn show_cursor(&mut self) -> Result {
		self.cursor_hidden = false;
		Ok(())
	}

	fn get_cursor(&mut self) -> Result<UVec2> { Ok(self.cursor_position) }

	fn set_cursor(&mut self, position: UVec2) -> Result {
		self.cursor_position = position;
		Ok(())
	}

	fn clear(&mut self) -> Result {
		self.buffer.clear();
		Ok(())
	}

	fn window_size(&mut self) -> Result<WindowSize> {
		Ok(WindowSize {
			chars: self.size(),
			pixels: UVec2::ZERO,
		})
	}

	fn draw(&mut self, buffer: &Buffer) -> Result {
		self.buffer = buffer.clone();
		Ok(())
	}

	fn flush(&mut self) -> Result { Ok(()) }
}
