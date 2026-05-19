use crate::render::DoubleBuffer;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// Returns the display width (in terminal columns) for a character.
///
/// Wide characters (CJK, fullwidth) return 2; everything else returns 1.
pub(crate) fn unicode_width(c: char) -> u16 {
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

impl Default for Buffer {
	fn default() -> Self { Self::new(terminal_ext::size()) }
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

	pub fn new_half_terminal() -> Self {
		let size = terminal_ext::size();
		Self::new(UVec2::new(size.x, size.y / 2))
	}

	pub fn into_double_buffer(self) -> DoubleBuffer {
		DoubleBuffer::from_buffer(self)
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

	/// Write text starting at position, advancing by each character's display width.
	///
	/// Wide (CJK/fullwidth) characters occupy 2 columns; the trailing column is
	/// written as a `None`-symbol placeholder so the diff sees it as changed.
	pub fn write_text(
		&mut self,
		pos: UVec2,
		text: &str,
		style: VisualStyle,
		entity: Entity,
	) {
		let mut col = 0u32;
		for ch in text.chars() {
			let w = unicode_width(ch) as u32;
			let cell_pos = UVec2::new(pos.x + col, pos.y);
			if cell_pos.x >= self.size.x {
				break;
			}
			self.set(
				cell_pos,
				Cell::new(ch.to_string(), style.clone(), entity),
			);
			// placeholder for the trailing column of a wide character
			if w == 2 {
				let cont_pos = UVec2::new(pos.x + col + 1, pos.y);
				if cont_pos.x < self.size.x {
					self.set(cont_pos, Cell {
						symbol: None,
						style: style.clone(),
						entity: Some(entity),
					});
				}
			}
			col += w;
		}
	}

	/// Fill all cells in `rect` with the given `cell`.
	pub fn fill_rect(&mut self, rect: URect, cell: Cell) {
		for y in rect.min.y..rect.max.y {
			for x in rect.min.x..rect.max.x {
				self.set(UVec2::new(x, y), cell.clone());
			}
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
				let cell = &self.cells[idx];
				// skip trailing columns of wide characters
				if cell.is_wide_continuation() {
					continue;
				}
				result.push_str(cell.symbol_str());
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
				} else if cell.is_wide_continuation() {
					// trailing column of a wide char — emit nothing, the wide char
					// already advanced the cursor past this column.
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
	/// The entity that last wrote this cell. `None` for truly blank cells.
	pub entity: Option<Entity>,
}

impl Cell {
	/// A blank cell with no symbol and default style.
	pub const BLANK: Self = Self {
		symbol: None,
		style: VisualStyle::DEFAULT,
		entity: None,
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
			entity: Some(entity),
		}
	}

	/// Display character, defaulting to a space for blank cells.
	pub fn symbol_str(&self) -> &str { self.symbol.as_deref().unwrap_or(" ") }

	/// Returns `true` if this cell is the trailing placeholder of a wide character.
	///
	/// Wide-char continuation cells have no symbol but retain the entity that
	/// wrote them. Renderers should skip these cells entirely (the wide char
	/// occupies both columns).
	pub fn is_wide_continuation(&self) -> bool {
		self.symbol.is_none() && self.entity.is_some()
	}

	/// Display width in terminal columns. Wide chars (CJK, fullwidth) = 2.
	pub fn cell_width(&self) -> u16 {
		self.symbol
			.as_deref()
			.and_then(|s| s.chars().next())
			.map(unicode_width)
			.unwrap_or(1)
	}

	/// Visual equality: same symbol and same style. `None` != `Some(" ")`.
	///
	/// Entity is disregarded.
	pub fn visual_eq(&self, other: &Self) -> bool {
		match (&self.symbol, &other.symbol) {
			(None, None) => true,
			(Some(a), Some(b)) => a == b && self.style == other.style,
			_ => false,
		}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;
	use crate::style::*;

	/// Render a bundle to a 40×5 buffer and return the trimmed plain string.
	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(40, 5), bundle).trim_lines()
	}

	fn bordered() -> BoxStyle {
		BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)))
	}

	#[beet_core::test]
	fn underline_does_not_bleed_into_border() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Hello" },
			bordered(),
			VisualStyle {
				decoration_line: DecorationLine::underline(),
				..default()
			}
		)]));
		out.as_str().xpect_contains("┌");
		out.xpect_contains("Hello");
	}

	#[beet_core::test]
	fn strike_does_not_bleed_into_border() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Hi" },
			bordered(),
			VisualStyle {
				decoration_line: DecorationLine::line_through(),
				..default()
			}
		)]));
		out.as_str().xpect_contains("┌");
		out.xpect_contains("Hi");
	}

	#[beet_core::test]
	fn italic_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Italic" },
			VisualStyle {
				text_style: TextStyle::ITALIC,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[3m");
	}

	#[beet_core::test]
	fn bold_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Bold" },
			VisualStyle {
				text_style: TextStyle::BOLD,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[1m");
	}

	#[beet_core::test]
	fn blink_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Blink" },
			VisualStyle {
				text_style: TextStyle::BLINK,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[5m");
	}
}
