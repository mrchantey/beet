use super::escape;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// Returns the display width (in terminal columns) for a character.
///
/// Wide characters (CJK, fullwidth) return 2; everything else returns 1.
fn char_width(c: char) -> u16 {
	match c as u32 {
		0x1100..=0x115F
		| 0x2E80..=0x303E
		| 0x3040..=0xA4CF
		| 0xA960..=0xA97F
		| 0xAC00..=0xD7FF
		| 0xF900..=0xFAFF
		| 0xFE10..=0xFE1F
		| 0xFE30..=0xFE4F
		| 0xFF01..=0xFF60
		| 0xFFE0..=0xFFE6
		| 0x1F300..=0x1FAFF => 2,
		_ => 1,
	}
}

/// A rectangular buffer of cells, indexed by position.
#[derive(Clone)]
pub struct Buffer {
	size: UVec2,
	cells: Vec<Cell>,
}

impl Buffer {
	/// Create a new buffer filled with blank cells.
	pub fn new(size: UVec2) -> Self {
		let len = (size.x * size.y) as usize;
		Self {
			size,
			cells: alloc::vec::from_elem(Cell::BLANK, len),
		}
	}

	/// Reset all cells to [`Cell::BLANK`].
	pub fn clear(&mut self) {
		for cell in &mut self.cells {
			*cell = Cell::BLANK;
		}
	}

	/// Alias for [`Buffer::clear`].
	pub fn reset(&mut self) { self.clear(); }

	/// Buffer dimensions.
	pub fn size(&self) -> UVec2 { self.size }

	/// Raw cell slice.
	pub fn cells(&self) -> &[Cell] { &self.cells }

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
			self.cells[idx] = cell;
		}
	}

	/// Get a cell at the given position.
	///
	/// Returns `Some` even for blank cells; `None` only for out-of-bounds.
	pub fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.index(pos).map(|idx| &self.cells[idx])
	}

	/// Iterate over all non-blank cells with their positions.
	pub fn iter_cells(&self) -> impl Iterator<Item = (UVec2, &Cell)> + '_ {
		let width = self.size.x;
		self.cells
			.iter()
			.enumerate()
			.filter_map(move |(idx, cell)| {
				cell.symbol.as_ref().map(|_| {
					let x = idx as u32 % width;
					let y = idx as u32 / width;
					(UVec2::new(x, y), cell)
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

	/// Render the buffer to plain text (no ANSI styling).
	pub fn render_plain(&self) -> String {
		let width = self.size.x as usize;
		let height = self.size.y as usize;
		let mut result = String::with_capacity(self.cells.len());

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				result.push_str(self.cells[idx].symbol_str());
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
				let cell = &self.cells[idx];
				if cell.symbol.is_some() {
					escape::write_style(
						&mut out,
						&cell.style,
						prev_style.as_ref(),
					)
					// writing to vec<u8>, cannot fail
					.ok();
					out.extend_from_slice(cell.symbol_str().as_bytes());
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
	/// Symbol to display. `None` represents a blank/placeholder cell
	/// (eg. trailing cell of a wide unicode character), rendered as a space.
	pub symbol: Option<SmolStr>,
	pub style: VisualStyle,
	pub entity: Entity,
}

impl Cell {
	/// A blank cell with no symbol and default style.
	pub const BLANK: Self = Self {
		symbol: None,
		style: VisualStyle::DEFAULT,
		entity: Entity::PLACEHOLDER,
	};

	/// Create a cell with a symbol.
	pub fn new(
		symbol: impl Into<SmolStr>,
		style: impl Into<VisualStyle>,
		entity: Entity,
	) -> Self {
		Self {
			symbol: Some(symbol.into()),
			style: style.into(),
			entity,
		}
	}

	/// Display character, defaulting to a space for blank cells.
	pub fn symbol_str(&self) -> &str { self.symbol.as_deref().unwrap_or(" ") }

	/// Display width in terminal columns. Wide chars (CJK, fullwidth) = 2.
	pub fn cell_width(&self) -> u16 {
		self.symbol
			.as_deref()
			.and_then(|s| s.chars().next())
			.map(char_width)
			.unwrap_or(1)
	}

	/// Visual equality: same symbol (`None` == `" "`) and same style.
	///
	/// Entity is disregarded.
	pub fn visual_eq(&self, other: &Self) -> bool {
		self.symbol_str() == other.symbol_str() && self.style == other.style
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
		cell.set_symbol(self.symbol_str());
		cell.set_style(self.style.to_ratatui_style());
		cell
	}
}
