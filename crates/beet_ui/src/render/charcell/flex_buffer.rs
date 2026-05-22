use super::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// Sentinel viewport height for a [`FlexBuffer`].
///
/// Layout and measure see this as the available vertical space so a document is
/// never clipped, while the cell allocation only grows to the rows actually
/// painted. Kept well below `u32::MAX` so any (unexpected) percentage spacing
/// can't overflow the rem conversion.
pub const AUTO_GROW_VIEWPORT_HEIGHT: u32 = u16::MAX as u32;

/// An auto-growing buffer of fixed width and unbounded height, for stdout
/// output that must not clip to the terminal.
///
/// Layout sees a [sentinel](AUTO_GROW_VIEWPORT_HEIGHT) height; the backing rows
/// grow lazily as paints land and trailing blank rows are trimmed on render.
/// Unlike the fixed [`Buffer`] it can carry per-cell OSC-8 hyperlinks, kept in a
/// [separate map](Self::set_link) so [`Cell`] stays shared between both kinds.
#[derive(Component)]
pub struct FlexBuffer {
	width: u32,
	cells: Vec<Cell>,
	/// Per-position OSC-8 hyperlink targets, emitted around their cell's run by
	/// [`render`](Self::render).
	links: HashMap<UVec2, SmolStr>,
}

impl FlexBuffer {
	/// Create an empty auto-growing buffer of fixed `width`.
	pub fn new(width: u32) -> Self {
		Self {
			width,
			cells: Vec::new(),
			links: HashMap::default(),
		}
	}

	/// Raw cell slice.
	pub fn cells(&self) -> &[Cell] { &self.cells }

	/// Convert position to buffer index, bounds-checked against allocated rows.
	fn index(&self, pos: UVec2) -> Option<usize> {
		if pos.x >= self.width || pos.y >= self.allocated_rows() {
			return None;
		}
		Some((pos.y * self.width + pos.x) as usize)
	}

	/// Grow the backing rows so that row `rows - 1` is addressable, capped at
	/// the sentinel.
	fn ensure_rows(&mut self, rows: u32) {
		let rows = rows.min(AUTO_GROW_VIEWPORT_HEIGHT);
		let needed = (rows * self.width) as usize;
		if self.cells.len() < needed {
			self.cells.resize(needed, Cell::BLANK);
		}
	}

	/// Number of rows to render: the last non-blank row (+1), trimming trailing
	/// blank rows.
	fn render_height(&self) -> u32 {
		let width = self.width as usize;
		if width == 0 {
			return 0;
		}
		for row in (0..self.allocated_rows()).rev() {
			let start = row as usize * width;
			if self.cells[start..start + width]
				.iter()
				.any(|cell| cell.symbol.is_some())
			{
				return row + 1;
			}
		}
		0
	}

	/// Render to plain text (no ANSI styling), trimming trailing blank rows and
	/// per-line trailing spaces.
	pub fn render_plain(&self) -> String {
		trim_line_trailing(&render_cells_plain(
			&self.cells,
			self.width as usize,
			self.render_height() as usize,
		))
	}

	/// Render to a string with ANSI styling and OSC-8 hyperlinks, trimming
	/// trailing blank rows and per-line trailing spaces.
	pub fn render(&self) -> String {
		let links = &self.links;
		trim_line_trailing(&render_cells_ansi(
			&self.cells,
			self.width as usize,
			self.render_height() as usize,
			|pos| links.get(&pos).cloned(),
		))
	}
}

impl AsBuffer for FlexBuffer {
	fn size(&self) -> UVec2 { UVec2::new(self.width, AUTO_GROW_VIEWPORT_HEIGHT) }

	fn allocated_rows(&self) -> u32 {
		if self.width == 0 {
			0
		} else {
			self.cells.len() as u32 / self.width
		}
	}

	fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.index(pos).map(|idx| &self.cells[idx])
	}

	fn set(&mut self, pos: UVec2, cell: Cell) {
		if pos.x >= self.width || pos.y >= AUTO_GROW_VIEWPORT_HEIGHT {
			return;
		}
		self.ensure_rows(pos.y + 1);
		if let Some(idx) = self.index(pos) {
			self.cells[idx] = cell;
		}
	}

	fn clear(&mut self) {
		self.cells.clear();
		self.links.clear();
	}

	fn set_link(&mut self, pos: UVec2, url: &str) {
		self.links.insert(pos, url.into());
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::VisualStyle;
	use bevy::math::URect;

	fn cell(symbol: &str) -> Cell {
		Cell::new(symbol, VisualStyle::default(), Entity::PLACEHOLDER)
	}

	#[beet_core::test]
	fn retains_rows_past_terminal() {
		// writing far below a terminal-sized viewport grows the allocation
		// instead of clipping the cell away.
		let mut buf = FlexBuffer::new(4);
		buf.write_text(
			UVec2::new(0, 500),
			"hi",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
		);
		buf.allocated_rows().xpect_eq(501);
		buf.get(UVec2::new(0, 500)).unwrap().symbol_str().xpect_eq("h");
	}

	#[beet_core::test]
	fn fill_rect_does_not_grow() {
		// a viewport-tall background must not balloon the allocation; fill is
		// clamped to the rows already painted (none here).
		let mut buf = FlexBuffer::new(4);
		buf.fill_rect(URect::new(0, 0, 4, AUTO_GROW_VIEWPORT_HEIGHT), cell("x"));
		buf.allocated_rows().xpect_eq(0);
	}

	#[beet_core::test]
	fn osc8_link_wraps_cell_run() {
		// a cell carrying a link is wrapped in OSC-8 open/close sequences,
		// closed again when the link clears on the following blank cell.
		let mut buf = FlexBuffer::new(6);
		buf.set(UVec2::new(0, 0), cell("x"));
		buf.set_link(UVec2::new(0, 0), "https://beetstack.dev");
		buf.render()
			.as_str()
			.xpect_contains("\x1b]8;;https://beetstack.dev\x1b\\")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[beet_core::test]
	fn trims_trailing_blank_rows() {
		// content on row 0, blank intermediate rows preserved up to a blanked
		// trailing row → render trims back to the single content row.
		let mut buf = FlexBuffer::new(4);
		buf.write_text(
			UVec2::new(0, 0),
			"hi",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
		);
		buf.write_text(
			UVec2::new(0, 3),
			"yo",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
		);
		buf.allocated_rows().xpect_eq(4);
		// blank out the trailing content row
		for x in 0..4 {
			buf.set(UVec2::new(x, 3), Cell::BLANK);
		}
		buf.render_plain().lines().count().xpect_eq(1);
	}
}
