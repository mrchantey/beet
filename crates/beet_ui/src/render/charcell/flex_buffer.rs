use super::*;
use crate::prelude::MediaViewport;
use beet_core::prelude::*;
use bevy::math::IRect;
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
/// OSC-8 hyperlinks ride on the [`Cell`] (see [`Cell::link`]), so the same
/// [`render_cells_ansi`] emits them here as on the fixed [`Buffer`].
///
/// Requires a [`MediaViewport`], kept in lockstep with the buffer width by
/// [`sync_media_viewport`](super::plugin), so width-gated media rules apply to
/// one-shot renders exactly as they do to live surfaces.
#[derive(Component)]
#[require(MediaViewport)]
pub struct FlexBuffer {
	width: u32,
	cells: Vec<Cell>,
}

impl FlexBuffer {
	/// Create an empty auto-growing buffer of fixed `width`.
	pub fn new(width: u32) -> Self {
		Self {
			width,
			cells: Vec::new(),
		}
	}

	/// Raw cell slice.
	pub fn cells(&self) -> &[Cell] { &self.cells }

	/// The OSC-8 hyperlink target attached to the cell at `pos`, if any.
	pub fn link_at(&self, pos: UVec2) -> Option<&str> {
		self.get(pos).and_then(|cell| cell.link.as_deref())
	}

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
	/// per-row trailing blank padding.
	pub fn render_plain(&self) -> String {
		render_cells_plain(
			&self.cells,
			self.width as usize,
			self.render_height() as usize,
		)
	}

	/// Render to a string with ANSI styling and OSC-8 hyperlinks, trimming
	/// trailing blank rows and per-row trailing blank padding.
	pub fn render(&self) -> String {
		render_cells_ansi(
			&self.cells,
			self.width as usize,
			self.render_height() as usize,
		)
	}
}

impl AsBuffer for FlexBuffer {
	fn size(&self) -> UVec2 {
		UVec2::new(self.width, AUTO_GROW_VIEWPORT_HEIGHT)
	}

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

	/// Fill the signed `rect` within `clip`, growing the backing rows to back a
	/// finite region.
	///
	/// A node paints its background before its content, so without growth a fill
	/// that lands above the current high-water mark (eg the app bar, painted
	/// pre-order before any text) would clamp to nothing. A sentinel-tall fill
	/// (the root viewport) is left to lazy growth so it can't balloon the
	/// allocation. Cells outside the clip (or above/left of the origin) are
	/// dropped.
	fn fill_rect(&mut self, rect: IRect, cell: Cell, clip: Clip) {
		let rect = clip.intersect(rect);
		let max_y = rect.max.y.max(0) as u32;
		if max_y < AUTO_GROW_VIEWPORT_HEIGHT {
			self.ensure_rows(max_y);
		}
		let max_y = max_y.min(self.allocated_rows());
		for y in rect.min.y.max(0) as u32..max_y {
			for x in rect.min.x.max(0) as u32..rect.max.x.max(0) as u32 {
				self.set(UVec2::new(x, y), cell.clone());
			}
		}
	}

	fn clear(&mut self) { self.cells.clear(); }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::VisualStyle;
	use bevy::math::IRect;
	use bevy::math::IVec2;

	fn cell(symbol: &str) -> Cell {
		Cell::new(symbol, VisualStyle::default(), Entity::PLACEHOLDER)
	}

	#[beet_core::test]
	fn retains_rows_past_terminal() {
		// writing far below a terminal-sized viewport grows the allocation
		// instead of clipping the cell away.
		let mut buf = FlexBuffer::new(4);
		buf.write_text(
			IVec2::new(0, 500),
			"hi",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
			Clip::NONE,
		);
		buf.allocated_rows().xpect_eq(501);
		buf.get(UVec2::new(0, 500))
			.unwrap()
			.symbol_str()
			.xpect_eq("h");
	}

	#[beet_core::test]
	fn sentinel_fill_does_not_grow() {
		// a viewport-tall (sentinel) background must not balloon the allocation;
		// such a fill is clamped to the rows already painted (none here).
		let mut buf = FlexBuffer::new(4);
		buf.fill_rect(
			IRect::new(0, 0, 4, AUTO_GROW_VIEWPORT_HEIGHT as i32),
			cell("x"),
			Clip::NONE,
		);
		buf.allocated_rows().xpect_eq(0);
	}

	#[beet_core::test]
	fn finite_fill_grows_to_back_region() {
		// a finite background (eg an elevated bar) painted before its content
		// grows the allocation so the fill is retained behind later text.
		let mut buf = FlexBuffer::new(4);
		buf.fill_rect(IRect::new(0, 0, 4, 2), cell("x"), Clip::NONE);
		buf.allocated_rows().xpect_eq(2);
		buf.get(UVec2::new(3, 1))
			.unwrap()
			.symbol_str()
			.xpect_eq("x");
	}

	#[beet_core::test]
	fn osc8_link_wraps_cell_run() {
		// a cell carrying a link is wrapped in OSC-8 open/close sequences,
		// closed again when the link clears on the following blank cell.
		let mut buf = FlexBuffer::new(6);
		buf.set(UVec2::new(0, 0), cell("x"));
		buf.set_link(UVec2::new(0, 0), "https://beet.org");
		buf.render()
			.as_str()
			.xpect_contains("\x1b]8;;https://beet.org\x1b\\")
			.xpect_contains("\x1b]8;;\x1b\\");
	}

	#[beet_core::test]
	fn trims_trailing_blank_rows() {
		// content on row 0, blank intermediate rows preserved up to a blanked
		// trailing row → render trims back to the single content row.
		let mut buf = FlexBuffer::new(4);
		buf.write_text(
			IVec2::new(0, 0),
			"hi",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
			Clip::NONE,
		);
		buf.write_text(
			IVec2::new(0, 3),
			"yo",
			VisualStyle::default(),
			Entity::PLACEHOLDER,
			Clip::NONE,
		);
		buf.allocated_rows().xpect_eq(4);
		// blank out the trailing content row
		for x in 0..4 {
			buf.set(UVec2::new(x, 3), Cell::BLANK);
		}
		buf.render_plain().lines().count().xpect_eq(1);
	}
}
