use super::*;
use bevy::color::Color;
use bevy::math::URect;
use bevy::math::UVec2;

// ── Cell & VisualStyle ────────────────────────────────────────────────────────

/// Visual styling for a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualStyle {
	pub foreground: Option<Color>,
	pub background: Option<Color>,
	pub underline: Option<Color>,
}

impl Default for VisualStyle {
	fn default() -> Self {
		Self {
			foreground: None,
			background: None,
			underline: None,
		}
	}
}

/// A single terminal cell with text and styling.
#[derive(Debug, Clone)]
pub struct Cell {
	pub symbol: String,
	pub style: VisualStyle,
}

impl Cell {
	pub fn new(symbol: impl Into<String>) -> Self {
		Self {
			symbol: symbol.into(),
			style: VisualStyle::default(),
		}
	}

	pub fn with_style(mut self, style: VisualStyle) -> Self {
		self.style = style;
		self
	}

	pub fn with_fg(mut self, color: Color) -> Self {
		self.style.foreground = Some(color);
		self
	}

	pub fn with_bg(mut self, color: Color) -> Self {
		self.style.background = Some(color);
		self
	}
}

// ── Buffer ────────────────────────────────────────────────────────────────────

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
			cells: vec![None; size],
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
	pub fn write_text(&mut self, pos: UVec2, text: &str, style: VisualStyle) {
		for (i, ch) in text.chars().enumerate() {
			let cell_pos = UVec2::new(pos.x + i as u32, pos.y);
			if cell_pos.x >= self.rect.max.x {
				break;
			}
			self.set(cell_pos, Cell {
				symbol: ch.to_string(),
				style: style.clone(),
			});
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
}

// ── Widget trait ──────────────────────────────────────────────────────────────

pub trait Widget {
	fn layout_style(&self) -> &LayoutStyle;

	/// Pass 1 (bottom-up): given available space as a hint, return desired size.
	fn measure(&self, available: UVec2) -> UVec2;

	/// Pass 2 (top-down): given the assigned rect, emit render cells to buffer.
	fn layout(&self, rect: URect, buffer: &mut Buffer);
}
