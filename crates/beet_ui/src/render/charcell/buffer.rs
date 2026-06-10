use crate::render::DoubleBuffer;
#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
use crate::style::BlinkStyle;
#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
use crate::style::FontStyle;
#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
use crate::style::Visibility;
use crate::style::VisualStyle;
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

/// Shared interface over the fixed [`Buffer`], the auto-growing [`FlexBuffer`],
/// and [`DoubleBuffer`].
///
/// Paint helpers take `&mut impl AsBuffer` so the same box-model and text-flow
/// code drives any backing store. Implementors provide cell storage; the text
/// and rectangle writers are derived from it.
pub trait AsBuffer {
	/// Logical size available to layout. For a [`FlexBuffer`] the height is a
	/// sentinel rather than the allocated row count.
	fn size(&self) -> UVec2;

	/// Number of rows currently backed by cells. Equals `size().y` for fixed
	/// buffers; grows with paints for a [`FlexBuffer`].
	fn allocated_rows(&self) -> u32;

	/// Cell at `pos`, or `None` when out of the allocated bounds.
	fn get(&self, pos: UVec2) -> Option<&Cell>;

	/// Write a cell at `pos`. Out-of-bounds writes are dropped.
	fn set(&mut self, pos: UVec2, cell: Cell);

	/// Set a cell, composing it over the destination: a cell with no background
	/// of its own keeps the background already present. Terminal cells are
	/// opaque, so this is how a glyph or border shows the fill beneath it rather
	/// than punching a transparent hole in it.
	fn set_composite(&mut self, pos: UVec2, mut cell: Cell) {
		if cell.style.background.is_none() {
			cell.style.background =
				self.get(pos).and_then(|existing| existing.style.background);
		}
		self.set(pos, cell);
	}

	/// Reset all cells to [`Cell::BLANK`].
	fn clear(&mut self);

	/// Alias for [`clear`](Self::clear).
	fn reset(&mut self) { self.clear(); }

	/// Attach an OSC-8 hyperlink target to the cell at `pos`.
	///
	/// A no-op for backings that don't carry links — the fixed [`Buffer`] and
	/// the TUI [`DoubleBuffer`], which route clicks through their own handlers.
	/// Only the stdout [`FlexBuffer`] records and emits links.
	fn set_link(&mut self, _pos: UVec2, _url: &str) {}

	/// Write text starting at `pos`, advancing by each character's display width.
	///
	/// Wide (CJK/fullwidth) characters occupy 2 columns; the trailing column is
	/// written as a `None`-symbol placeholder so the diff sees it as changed.
	fn write_text(
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
			// a wide glyph displays 2 columns, so stop before one that can't fit
			// both rather than overflowing the right edge by a column.
			if cell_pos.x + w > self.size().x {
				break;
			}
			// glyphs compose over the cell beneath them (see `set_composite`),
			// keeping the page/code surface fill rather than punching a hole.
			self.set_composite(
				cell_pos,
				Cell::new(ch.to_string(), style.clone(), entity),
			);
			// placeholder for the trailing column of a wide character
			if w == 2 {
				self.set_composite(UVec2::new(cell_pos.x + 1, pos.y), Cell {
					symbol: None,
					style: style.clone(),
					entity: Some(entity),
				});
			}
			col += w;
		}
	}

	/// Fill all cells in `rect` with `cell`.
	///
	/// Clamped to the [allocated rows](Self::allocated_rows) so a sentinel-tall
	/// background can't explode a [`FlexBuffer`] allocation; only painted rows
	/// are filled.
	fn fill_rect(&mut self, rect: URect, cell: Cell) {
		let max_y = rect.max.y.min(self.allocated_rows());
		for y in rect.min.y..max_y {
			for x in rect.min.x..rect.max.x {
				self.set(UVec2::new(x, y), cell.clone());
			}
		}
	}
}

/// A fixed `width × height` rectangular grid of cells, indexed by position.
///
/// The terminal-sized buffer used by the TUI path (via [`DoubleBuffer`]) and by
/// fixed-size one-shot rendering. For unbounded stdout output see [`FlexBuffer`].
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

	/// Raw cell slice.
	pub fn cells(&self) -> &[Cell] { &self.cells }

	/// Convert position to buffer index, or `None` when out of bounds.
	fn index(&self, pos: UVec2) -> Option<usize> {
		if pos.x >= self.size.x || pos.y >= self.size.y {
			return None;
		}
		Some((pos.y * self.size.x + pos.x) as usize)
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

	/// Render the buffer to plain text (no ANSI styling).
	pub fn render_plain(&self) -> String {
		render_cells_plain(
			&self.cells,
			self.size.x as usize,
			self.size.y as usize,
		)
	}

	/// Render the buffer to a string with ANSI styling.
	pub fn render(&self) -> String {
		render_cells_ansi(
			&self.cells,
			self.size.x as usize,
			self.size.y as usize,
			|_| None,
		)
	}
}

impl AsBuffer for Buffer {
	fn size(&self) -> UVec2 { self.size }

	fn allocated_rows(&self) -> u32 { self.size.y }

	fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.index(pos).map(|idx| &self.cells[idx])
	}

	fn set(&mut self, pos: UVec2, cell: Cell) {
		if let Some(idx) = self.index(pos) {
			self.cells[idx] = cell;
		}
	}

	fn clear(&mut self) {
		for cell in &mut self.cells {
			*cell = Cell::BLANK;
		}
	}
}

// ── Rendering ───────────────────────────────────────────────────────────────

/// Exclusive end column for a row: one past its last [significant](Cell::is_significant)
/// cell, so trailing blank padding is dropped while background fills are kept.
fn row_render_end(cells: &[Cell], width: usize, y: usize) -> usize {
	let row = &cells[y * width..y * width + width];
	row.iter()
		.rposition(Cell::is_significant)
		.map(|idx| idx + 1)
		.unwrap_or(0)
}

/// Render a row-major cell grid (`width × height`) to plain text.
///
/// Trailing blank padding is trimmed per row (see [`row_render_end`]) so rows
/// render ragged rather than padded to the full width.
pub(crate) fn render_cells_plain(
	cells: &[Cell],
	width: usize,
	height: usize,
) -> String {
	let mut result = String::with_capacity(cells.len());
	for y in 0..height {
		for x in 0..row_render_end(cells, width, y) {
			let cell = &cells[y * width + x];
			// skip trailing columns of wide characters
			if cell.is_wide_continuation() {
				continue;
			}
			result.push_str(cell.symbol_str());
		}
		if y + 1 < height {
			result.push('\n');
		}
	}
	result
}

/// Render a row-major cell grid to a string with ANSI styling.
///
/// `link_at` supplies an optional [OSC-8 hyperlink](https://gist.github.com/egmontkob/eb114294efbcd5adb1944c9f3cb5feda)
/// target per position — always `None` for the fixed [`Buffer`], the links map
/// for a [`FlexBuffer`]. Links are emitted only by this stdout backend, not the
/// ratatui/TUI path which routes clicks through its own handlers.
pub(crate) fn render_cells_ansi(
	cells: &[Cell],
	width: usize,
	height: usize,
	mut link_at: impl FnMut(UVec2) -> Option<SmolStr>,
) -> String {
	let mut out: Vec<u8> = Vec::with_capacity(cells.len() * 8);
	let mut prev_style: Option<VisualStyle> = None;
	let mut prev_link: Option<SmolStr> = None;

	for y in 0..height {
		for x in 0..row_render_end(cells, width, y) {
			let cell = &cells[y * width + x];
			// trailing column of a wide char emits nothing and keeps the
			// current link/style state intact.
			if cell.is_wide_continuation() {
				continue;
			}
			// open/close OSC-8 hyperlinks as the active link changes
			let link = if cell.symbol.is_some() {
				link_at(UVec2::new(x as u32, y as u32))
			} else {
				None
			};
			if link != prev_link {
				write_osc8(&mut out, link.as_deref());
				prev_link = link;
			}

			if cell.symbol.is_some() {
				cell.style
					.write_style(&mut out, prev_style.as_ref())
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
		// reset at the end of each row so an active background (eg an app bar
		// fill reaching the edge) can't bleed across the newline via the
		// terminal's back-colour-erase.
		if prev_style.is_some() {
			out.extend_from_slice(escape::RESET.as_bytes());
			prev_style = None;
		}
		if prev_link.is_some() {
			write_osc8(&mut out, None);
			prev_link = None;
		}
		if y + 1 < height {
			out.push(b'\n');
		}
	}
	String::from_utf8_lossy(&out).into_owned()
}

/// Write an OSC-8 hyperlink sequence: opening with `url`, or closing for `None`.
fn write_osc8(out: &mut Vec<u8>, url: Option<&str>) {
	out.extend_from_slice(escape::OSC8_LINK.as_bytes());
	if let Some(url) = url {
		out.extend_from_slice(url.as_bytes());
	}
	out.extend_from_slice(escape::ST.as_bytes());
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

	/// Whether the cell must be emitted rather than trimmed as trailing padding.
	///
	/// A glyph other than a blank space is always significant, as is any cell
	/// carrying a background fill (so a full-width app bar or code surface keeps
	/// its colour to the edge). Truly blank padding (`Cell::BLANK` or an
	/// unstyled space) is trimmed from the end of each row.
	pub fn is_significant(&self) -> bool {
		self.symbol.as_deref().is_some_and(|symbol| symbol != " ")
			|| self.style.background.is_some()
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
#[extend::ext]
impl VisualStyle {
	/// Converts to a ratatui [`Style`](ratatui::style::Style).
	fn to_ratatui_style(&self) -> ratatui::style::Style {
		let mut modifier = ratatui::style::Modifier::empty();
		if self.font_weight.is_bold() {
			modifier |= ratatui::style::Modifier::BOLD;
		}
		if self.font_style == FontStyle::Italic {
			modifier |= ratatui::style::Modifier::ITALIC;
		}
		// dim derived from foreground alpha
		if let Some(fg) = self.foreground {
			if fg.to_srgba_u8().alpha < 128 {
				modifier |= ratatui::style::Modifier::DIM;
			}
		}
		match self.blink {
			BlinkStyle::None => {}
			BlinkStyle::Blink => {
				modifier |= ratatui::style::Modifier::SLOW_BLINK
			}
			BlinkStyle::RapidBlink => {
				modifier |= ratatui::style::Modifier::RAPID_BLINK
			}
		}
		if self.visibility == Visibility::Hidden {
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
	use crate::style::FontWeight;
	use crate::style::*;

	/// Render a bundle to a 40×5 buffer and return the trimmed plain string.
	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(40, 5), bundle).trim_lines()
	}

	fn bordered() -> BoxStyle {
		BoxStyle::default().with_border(Spacing::all(Length::Px(1.)))
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
				font_style: FontStyle::Italic,
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
				font_weight: FontWeight::Bold,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[1m");
	}

	#[beet_core::test]
	fn full_box_border_carries_fill_background() {
		// a bordered box with a background clips the fill to the border edge, so
		// its corner glyph sits on the box's own surface, not the page beneath.
		let bg = Color::srgb(0.5, 0., 0.5);
		let buffer = Buffer::new(UVec2::new(10, 4)).populate((
			LayoutStyle::flex_row(),
			children![(rsx! { "X" }, bordered(), VisualStyle {
				background: Some(bg),
				..default()
			})],
		));
		let corner = buffer.get(UVec2::new(0, 0)).unwrap();
		corner.symbol_str().xpect_eq("┌");
		corner.style.background.xpect_eq(Some(bg));
	}

	#[beet_core::test]
	fn divider_carries_node_background() {
		// a bottom-border-only node (eg an app bar / footer) renders its divider
		// in its own surface colour, so the edge reads as part of the bar rather
		// than the page beneath it.
		let bg = Color::srgb(0.5, 0., 0.5);
		let buffer = Buffer::new(UVec2::new(10, 4)).populate((
			LayoutStyle::flex_row(),
			children![(
				rsx! { "Bar" },
				BoxStyle::default().with_border(Spacing {
					bottom: Length::Px(1.),
					..Spacing::DEFAULT
				}),
				VisualStyle {
					background: Some(bg),
					..default()
				}
			)],
		));
		buffer
			.iter_cells()
			.find(|(_, cell)| cell.symbol_str() == "─")
			.unwrap()
			.1
			.style
			.background
			.xpect_eq(Some(bg));
	}

	#[beet_core::test]
	fn blink_renders() {
		let out = render((LayoutStyle::flex_row(), children![(
			rsx! { "Blink" },
			VisualStyle {
				blink: BlinkStyle::Blink,
				..default()
			}
		)]));
		out.xpect_contains("\x1b[5m");
	}
}
