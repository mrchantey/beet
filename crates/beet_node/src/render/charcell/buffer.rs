use crate::style::TextDecoration;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// A rectangular buffer of cells, indexed by position.
pub struct Buffer {
	rect: URect,
	cells: Vec<Option<Cell>>,
}

impl Buffer {
	pub fn new(rect: URect) -> Self {
		let size = (rect.width() * rect.height()) as usize;
		Self {
			rect,
			cells: alloc::vec::from_elem(None, size),
		}
	}

	pub fn rect(&self) -> URect { self.rect }

	/// Convert position to buffer index.
	fn index(&self, pos: UVec2) -> Option<usize> {
		if pos.x < self.rect.min.x
			|| pos.y < self.rect.min.y
			|| pos.x >= self.rect.max.x
			|| pos.y >= self.rect.max.y
		{
			return None;
		}
		let local_x = pos.x - self.rect.min.x;
		let local_y = pos.y - self.rect.min.y;
		Some((local_y * self.rect.width() + local_x) as usize)
	}

	/// Set a cell at the given position.
	pub fn set(&mut self, pos: UVec2, cell: Cell) {
		if let Some(idx) = self.index(pos) {
			self.cells[idx] = Some(cell);
		}
	}

	/// Get a cell at the given position.
	pub fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.index(pos).and_then(|idx| self.cells[idx].as_ref())
	}

	/// Write text starting at position, wrapping each character into a cell.
	pub fn write_text(
		&mut self,
		pos: UVec2,
		text: &str,
		style: impl Clone + Into<CharStyle>,
	) {
		for (i, ch) in text.chars().enumerate() {
			let cell_pos = UVec2::new(pos.x + i as u32, pos.y);
			if cell_pos.x >= self.rect.max.x {
				break;
			}
			self.set(cell_pos, Cell::new(ch.to_string(), style.clone()));
		}
	}

	/// Render the buffer to a string (plain text, no styling).
	pub fn render_plain(&self) -> String {
		let width = self.rect.width() as usize;
		let height = self.rect.height() as usize;
		let mut result = String::with_capacity(self.cells.len());

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				if let Some(cell) = &self.cells[idx] {
					result.push_str(&cell.symbol);
				} else {
					result.push(' ');
				}
			}
			if y < height - 1 {
				result.push('\n');
			}
		}
		result
	}

	pub fn render_plain_trim(&self) -> String {
		self.render_plain().trim_start_lines().trim_end_lines()
	}
}


/// A single terminal cell with text and styling.
#[derive(Debug, Clone, SetWith)]
pub struct Cell {
	pub symbol: SmolStr,
	#[set_with(into)]
	pub style: CharStyle,
}

impl Cell {
	pub fn new(
		symbol: impl Into<SmolStr>,
		style: impl Into<CharStyle>,
	) -> Self {
		Self {
			symbol: symbol.into(),
			style: style.into(),
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, SetWith)]
pub struct CharStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attributes
	pub foreground: Option<Color>,
	pub background: Option<Color>,
	pub decoration_color: Option<Color>,
	pub decoration_line: Vec<TextDecoration>,
}

impl From<VisualStyle> for CharStyle {
	fn from(style: VisualStyle) -> Self {
		Self {
			foreground: style.foreground,
			background: style.background,
			decoration_color: style.decoration_color,
			decoration_line: style.decoration_line,
		}
	}
}
