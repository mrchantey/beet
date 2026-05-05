use crate::prelude::*;
use beet_core::prelude::*;

/// Wraps any [`ratatui::backend::Backend`] to implement beet's [`Backend`] trait.
pub struct RatatuiBackend<T: ratatui::backend::Backend> {
	inner: T,
}

impl<T: ratatui::backend::Backend> RatatuiBackend<T> {
	pub fn new(inner: T) -> Self { Self { inner } }
}

impl<T: ratatui::backend::Backend> Backend for RatatuiBackend<T> {
	fn hide_cursor(&mut self) -> Result { self.inner.hide_cursor()?.xok() }

	fn show_cursor(&mut self) -> Result { self.inner.show_cursor()?.xok() }

	fn get_cursor(&mut self) -> Result<UVec2> {
		let pos = self.inner.get_cursor_position()?;
		UVec2::new(pos.x as u32, pos.y as u32).xok()
	}

	fn set_cursor(&mut self, position: UVec2) -> Result {
		self.inner.set_cursor_position(ratatui::layout::Position {
			x: position.x as u16,
			y: position.y as u16,
		})?;
		Ok(())
	}

	fn clear(&mut self) -> Result { self.inner.clear()?.xok() }

	fn window_size(&mut self) -> Result<WindowSize> {
		let size = self.inner.window_size()?;
		WindowSize {
			chars: UVec2::new(
				size.columns_rows.width as u32,
				size.columns_rows.height as u32,
			),
			pixels: UVec2::new(
				size.pixels.width as u32,
				size.pixels.height as u32,
			),
		}
		.xok()
	}

	fn draw(&mut self, buffer: &Buffer) -> Result {
		let width = buffer.size().x;
		let height = buffer.size().y;
		// build ratatui cells from our buffer
		let mut cells: Vec<(u16, u16, ratatui::buffer::Cell)> = Vec::new();
		for idx in 0..(width * height) {
			let x = idx % width;
			let y = idx / width;
			if let Some(cell) = buffer.get(UVec2::new(x, y)) {
				cells.push((x as u16, y as u16, cell.to_ratatui_cell()));
			}
		}
		self.inner
			.draw(cells.iter().map(|(x, y, cell)| (*x, *y, cell)))?
			.xok()
	}

	fn flush(&mut self) -> Result { self.inner.flush()?.xok() }
}
