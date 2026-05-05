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
	fn hide_cursor(&mut self) -> Result {
		self.inner.hide_cursor().map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}

	fn show_cursor(&mut self) -> Result {
		self.inner.show_cursor().map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}

	fn get_cursor(&mut self) -> Result<UVec2> {
		let pos = self
			.inner
			.get_cursor_position()
			.map_err(|e| bevyhow!("{e}"))?;
		Ok(UVec2::new(pos.x as u32, pos.y as u32))
	}

	fn set_cursor(&mut self, position: UVec2) -> Result {
		self.inner
			.set_cursor_position(ratatui::layout::Position {
				x: position.x as u16,
				y: position.y as u16,
			})
			.map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}

	fn clear(&mut self) -> Result {
		self.inner.clear().map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}

	fn window_size(&mut self) -> Result<WindowSize> {
		let size = self.inner.window_size().map_err(|e| bevyhow!("{e}"))?;
		Ok(WindowSize {
			chars: UVec2::new(
				size.columns_rows.width as u32,
				size.columns_rows.height as u32,
			),
			pixels: UVec2::new(
				size.pixels.width as u32,
				size.pixels.height as u32,
			),
		})
	}

	fn draw(&mut self, buffer: &Buffer) -> Result {
		let width = buffer.size().x;
		let height = buffer.size().y;
		// Build ratatui cells from our buffer, skipping empty cells
		let cells: Vec<(u16, u16, ratatui::buffer::Cell)> = (0..height)
			.flat_map(|y| (0..width).map(move |x| (x, y)))
			.filter_map(|(x, y)| {
				buffer.get(UVec2::new(x, y)).map(|cell| {
					let mut ratatui_cell = ratatui::buffer::Cell::default();
					ratatui_cell.set_symbol(cell.symbol.as_str());
					(x as u16, y as u16, ratatui_cell)
				})
			})
			.collect();
		self.inner
			.draw(cells.iter().map(|(x, y, cell)| (*x, *y, cell)))
			.map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}

	fn flush(&mut self) -> Result {
		self.inner.flush().map_err(|e| bevyhow!("{e}"))?;
		Ok(())
	}
}
