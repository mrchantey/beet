use crate::prelude::*;
use beet_core::prelude::*;
use crossterm::cursor;
use crossterm::queue;
use crossterm::style::Attribute;
use crossterm::style::Attributes;
use crossterm::style::Color as CrosstermColor;
use crossterm::style::Colors;
use crossterm::style::Print;
use crossterm::style::SetAttribute;
use crossterm::style::SetAttributes;
use crossterm::style::SetColors;
use crossterm::terminal;
use std::io::Write;

/// Terminal backend that writes ANSI escape sequences via crossterm.
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
		crossterm::execute!(self.writer, cursor::Hide)?;
		Ok(())
	}

	fn show_cursor(&mut self) -> Result {
		crossterm::execute!(self.writer, cursor::Show)?;
		Ok(())
	}

	fn get_cursor(&mut self) -> Result<UVec2> {
		let (x, y) = cursor::position()?;
		UVec2::new(x as u32, y as u32).xok()
	}

	fn set_cursor(&mut self, position: UVec2) -> Result {
		crossterm::execute!(
			self.writer,
			cursor::MoveTo(position.x as u16, position.y as u16)
		)?;
		Ok(())
	}

	fn clear(&mut self) -> Result {
		crossterm::execute!(
			self.writer,
			terminal::Clear(terminal::ClearType::All)
		)?;
		Ok(())
	}

	fn window_size(&mut self) -> Result<WindowSize> {
		// pixel dimensions may not be available on all platforms
		match terminal::window_size() {
			Ok(window) => WindowSize {
				chars: UVec2::new(window.columns as u32, window.rows as u32),
				pixels: UVec2::new(window.width as u32, window.height as u32),
			},
			Err(_) => {
				// fallback to size
				let (cols, rows) = terminal::size()?;
				WindowSize {
					chars: UVec2::new(cols as u32, rows as u32),
					pixels: UVec2::ZERO,
				}
			}
		}
		.xok()
	}

	fn draw(&mut self, buffer: &Buffer) -> Result {
		let width = buffer.size().x;
		let height = buffer.size().y;
		let mut last_pos: Option<(u16, u16)> = None;
		let mut cur_fg = CrosstermColor::Reset;
		let mut cur_bg = CrosstermColor::Reset;
		let mut cur_attrs = Attributes::default();

		for y in 0..height {
			for x in 0..width {
				let Some(cell) = buffer.get(UVec2::new(x, y)) else {
					// gap in the buffer; force a MoveTo on the next cell
					last_pos = None;
					continue;
				};
				let (cx, cy) = (x as u16, y as u16);

				// skip MoveTo when directly following the previous cell
				if !matches!(last_pos, Some((lx, ly)) if cx == lx + 1 && cy == ly)
				{
					queue!(self.writer, cursor::MoveTo(cx, cy))?;
				}
				last_pos = Some((cx, cy));

				// apply color changes
				let content_style = cell.style.to_crossterm_content_style();
				let new_fg = content_style
					.foreground_color
					.unwrap_or(CrosstermColor::Reset);
				let new_bg = content_style
					.background_color
					.unwrap_or(CrosstermColor::Reset);
				if new_fg != cur_fg || new_bg != cur_bg {
					queue!(
						self.writer,
						SetColors(Colors::new(new_fg, new_bg))
					)?;
					cur_fg = new_fg;
					cur_bg = new_bg;
				}

				// apply attribute changes
				let new_attrs = content_style.attributes;
				if new_attrs != cur_attrs {
					queue!(self.writer, SetAttribute(Attribute::Reset))?;
					if !new_attrs.is_empty() {
						queue!(self.writer, SetAttributes(new_attrs))?;
					}
					cur_attrs = new_attrs;
				}

				queue!(self.writer, Print(cell.symbol.as_str()))?;
			}
		}

		// reset terminal state after drawing
		queue!(
			self.writer,
			SetColors(Colors::new(
				CrosstermColor::Reset,
				CrosstermColor::Reset,
			)),
			SetAttribute(Attribute::Reset),
		)?;
		Ok(())
	}

	fn flush(&mut self) -> Result {
		self.writer.flush()?;
		Ok(())
	}
}
