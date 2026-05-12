use super::escape;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// A rectangular buffer of cells, indexed by position.
#[derive(Clone)]
pub struct Buffer {
	size: UVec2,
	cells: Vec<Option<Cell>>,
}

impl Buffer {
	pub fn new(size: UVec2) -> Self {
		let len = (size.x * size.y) as usize;
		Self {
			size,
			cells: alloc::vec::from_elem(None, len),
		}
	}
	pub fn clear(&mut self) {
		for cell in &mut self.cells {
			*cell = None;
		}
	}

	pub fn size(&self) -> UVec2 { self.size }

	/// Convert position to buffer index.
	fn index(&self, pos: UVec2) -> Option<usize> {
		if pos.x >= self.size.x || pos.y >= self.size.y {
			return None;
		}
		Some((pos.y * self.size.x + pos.x) as usize)
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

	/// Iterate over all non-empty cells with their positions.
	pub fn iter_cells(&self) -> impl Iterator<Item = (UVec2, &Cell)> + '_ {
		let width = self.size.x;
		self.cells
			.iter()
			.enumerate()
			.filter_map(move |(idx, cell)| {
				cell.as_ref().map(|c| {
					let x = idx as u32 % width;
					let y = idx as u32 / width;
					(UVec2::new(x, y), c)
				})
			})
	}

	/// Write text starting at position, wrapping each character into a cell.
	pub fn write_text(
		&mut self,
		pos: UVec2,
		text: &str,
		style: VisualStyle,
		entity: Entity,
	) {
		for (i, ch) in text.chars().enumerate() {
			let cell_pos = UVec2::new(pos.x + i as u32, pos.y);
			if cell_pos.x >= self.size.x {
				break;
			}
			self.set(
				cell_pos,
				Cell::new(ch.to_string(), style.clone(), entity),
			);
		}
	}

	/// Render the buffer to a string (plain text, no styling).
	pub fn render_plain(&self) -> String {
		let width = self.size.x as usize;
		let height = self.size.y as usize;
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

	/// Render the buffer to a string with ANSI styling.
	pub fn render(&self) -> String {
		let width = self.size.x as usize;
		let height = self.size.y as usize;
		let mut out: Vec<u8> = Vec::with_capacity(self.cells.len() * 8);
		let mut prev_style: Option<VisualStyle> = None;

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				if let Some(cell) = &self.cells[idx] {
					escape::write_style(
						&mut out,
						&cell.style,
						prev_style.as_ref(),
					)
					// writing to vec<u8>
					.ok();
					out.extend_from_slice(cell.symbol.as_bytes());
					prev_style = Some(cell.style.clone());
				} else {
					if prev_style.is_some() {
						out.extend_from_slice(escape::RESET.as_bytes());
						prev_style = None;
					}
					out.push(b' ');
				}
			}
			if y < height - 1 {
				out.push(b'\n');
			}
		}
		if prev_style.is_some() {
			out.extend_from_slice(escape::RESET.as_bytes());
		}
		String::from_utf8_lossy(&out).into_owned()
	}
}

/// A single terminal cell with text and styling.
#[derive(Debug, Clone, SetWith)]
pub struct Cell {
	pub symbol: SmolStr,
	pub style: VisualStyle,
	pub entity: Entity,
}

impl Cell {
	pub fn new(
		symbol: impl Into<SmolStr>,
		style: impl Into<VisualStyle>,
		entity: Entity,
	) -> Self {
		Self {
			symbol: symbol.into(),
			style: style.into(),
			entity,
		}
	}
	/// Two cells are visually equal if their symbol and style match.
	/// The entity is disregarded
	pub fn visual_eq(&self, other: &Self) -> bool {
		self.symbol == other.symbol && self.style == other.style
	}
}



// ── Ratatui conversions ───────────────────────────────────────────────────────

/// Convert a bevy [`Color`] to a ratatui terminal color via RGB.
#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
fn color_to_ratatui(color: Color) -> ratatui::style::Color {
	let s = color.to_srgba_u8();
	ratatui::style::Color::Rgb(s.red, s.green, s.blue)
}

#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
impl VisualStyle {
	/// Converts to a ratatui [`Style`](ratatui::style::Style).
	pub fn to_ratatui_style(&self) -> ratatui::style::Style {
		let mut modifier = ratatui::style::Modifier::empty();
		let s = self.text_style;
		if s.contains(TextStyle::BOLD) {
			modifier |= ratatui::style::Modifier::BOLD;
		}
		if s.contains(TextStyle::ITALIC) {
			modifier |= ratatui::style::Modifier::ITALIC;
		}
		// dim derived from foreground alpha
		if let Some(fg) = self.foreground {
			if fg.to_srgba_u8().alpha < 128 {
				modifier |= ratatui::style::Modifier::DIM;
			}
		}
		if s.contains(TextStyle::BLINK) {
			modifier |= ratatui::style::Modifier::SLOW_BLINK;
		}
		if s.contains(TextStyle::RAPID_BLINK) {
			modifier |= ratatui::style::Modifier::RAPID_BLINK;
		}
		if s.contains(TextStyle::REVERSED) {
			modifier |= ratatui::style::Modifier::REVERSED;
		}
		if s.contains(TextStyle::HIDDEN) {
			modifier |= ratatui::style::Modifier::HIDDEN;
		}
		if self.decoration_line.underline {
			modifier |= ratatui::style::Modifier::UNDERLINED;
		}
		if self.decoration_line.line_through {
			modifier |= ratatui::style::Modifier::CROSSED_OUT;
		}
		// OVERLINE has no ratatui Modifier equivalent
		ratatui::style::Style {
			fg: self.foreground.map(color_to_ratatui),
			bg: self.background.map(color_to_ratatui),
			underline_color: self.decoration_color.map(color_to_ratatui),
			add_modifier: modifier,
			sub_modifier: ratatui::style::Modifier::empty(),
		}
	}
}

#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
impl Cell {
	/// Converts to a ratatui [`Cell`](ratatui::buffer::Cell).
	pub fn to_ratatui_cell(&self) -> ratatui::buffer::Cell {
		let mut cell = ratatui::buffer::Cell::default();
		cell.set_symbol(self.symbol.as_str());
		cell.set_style(self.style.to_ratatui_style());
		cell
	}
}
