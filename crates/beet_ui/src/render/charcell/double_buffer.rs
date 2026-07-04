use super::*;
use crate::prelude::MediaViewport;
use beet_core::prelude::*;
use bevy::math::UVec2;
use core::ops::Deref;
use core::ops::DerefMut;

/// Double-buffered terminal buffer.
///
/// Holds two [`Buffer`]s: one for the current frame being drawn,
/// one for the previous frame (used for diffing). Only changed cells
/// are written to the terminal on each render.
///
/// Requires a [`MediaViewport`], kept in lockstep with the buffer width by
/// [`sync_media_viewport`](super::plugin) — the width context width-gated
/// media rules resolve against, and the change-detection shadow that lets a
/// resize (and only a resize) re-run the style cascade.
#[derive(Component)]
#[require(MediaViewport)]
pub struct DoubleBuffer {
	buffers: [Buffer; 2],
	current: usize,
	/// Set by [`resize`](Self::resize), consumed by the terminal renderer via
	/// [`take_needs_clear`](Self::take_needs_clear) to erase the screen before
	/// the next draw.
	needs_clear: bool,
}

impl Default for DoubleBuffer {
	fn default() -> Self { Self::new(terminal_ext::size()) }
}

impl Deref for DoubleBuffer {
	type Target = Buffer;
	fn deref(&self) -> &Buffer { self.current_buffer() }
}

impl DerefMut for DoubleBuffer {
	fn deref_mut(&mut self) -> &mut Buffer { self.current_buffer_mut() }
}

impl DoubleBuffer {
	/// Create a new double buffer with the given terminal size.
	pub fn new(size: UVec2) -> Self {
		Self {
			buffers: [Buffer::new(size), Buffer::new(size)],
			current: 0,
			needs_clear: false,
		}
	}

	pub fn from_buffer(buffer: Buffer) -> Self {
		Self {
			buffers: [buffer.clone(), buffer],
			current: 0,
			needs_clear: false,
		}
	}

	pub fn into_buffer(self) -> Buffer {
		self.buffers.into_iter().nth(self.current).unwrap()
	}

	/// Reallocate both inner buffers to `size`, clearing them. On a terminal
	/// resize the old contents are invalid; both buffers start blank so the next
	/// paint diffs the full new frame against a cleared previous buffer and
	/// redraws everything. The renderer is flagged to erase the screen first
	/// (see [`take_needs_clear`](Self::take_needs_clear)).
	pub fn resize(&mut self, size: UVec2) {
		for buffer in &mut self.buffers {
			buffer.resize(size);
		}
		self.needs_clear = true;
	}

	/// Consume the pending-clear flag set by [`resize`](Self::resize).
	///
	/// The emulator reflows or pads the old screen content on resize, and the
	/// cell diff never touches cells that are blank in both buffers, so without a
	/// full erase stale glyphs survive wherever the new frame is blank.
	pub fn take_needs_clear(&mut self) -> bool {
		core::mem::take(&mut self.needs_clear)
	}

	/// The buffer currently being drawn into.
	pub fn current_buffer(&self) -> &Buffer { &self.buffers[self.current] }

	/// The most recently completed (on-screen) frame: the buffer not currently
	/// being drawn into. After [`swap_buffers`](Self::swap_buffers) this holds the
	/// frame just rendered to the terminal, so a test harness snapshots it to see
	/// what is on screen. Before the first swap it is the blank back buffer.
	pub fn front_buffer(&self) -> &Buffer { &self.buffers[1 - self.current] }

	/// Mutable access to the buffer currently being drawn into.
	pub fn current_buffer_mut(&mut self) -> &mut Buffer {
		&mut self.buffers[self.current]
	}

	/// Swap buffers: the current frame becomes the previous, and the old
	/// previous is reset for the next frame.
	pub fn swap_buffers(&mut self) {
		self.buffers[1 - self.current].reset();
		self.current = 1 - self.current;
	}

	/// Iterator over cells that changed since the last swap.
	///
	/// Compares the current buffer (new frame) against the previous buffer
	/// (on-screen state). Only yields cells that differ visually.
	///
	/// A changed wide-glyph trailing half is yielded as its owner glyph instead:
	/// the terminal draws both columns from the owner, and a placeholder has no
	/// glyph of its own to draw (a stale glyph there clobbered the owner's right
	/// half, so the owner must repaint).
	pub fn diff(&self) -> impl Iterator<Item = (UVec2, &Cell)> {
		let prev = &self.buffers[1 - self.current];
		let curr = &self.buffers[self.current];
		let width = curr.size().x;
		curr.cells()
			.iter()
			.enumerate()
			.filter_map(move |(i, cell)| {
				let changed =
					prev.cells().get(i).map_or(true, |p| !cell.visual_eq(p));
				if !changed {
					return None;
				}
				let pos = UVec2::new(i as u32 % width, i as u32 / width);
				if cell.is_wide_continuation() {
					// redraw the owner glyph one column left (same row only)
					return (pos.x > 0)
						.then(|| curr.cells().get(i - 1))
						.flatten()
						.filter(|owner| owner.cell_width() == 2)
						.map(|owner| (UVec2::new(pos.x - 1, pos.y), owner));
				}
				Some((pos, cell))
			})
	}
}

impl From<Buffer> for DoubleBuffer {
	fn from(buffer: Buffer) -> Self { Self::from_buffer(buffer) }
}
impl From<DoubleBuffer> for Buffer {
	fn from(db: DoubleBuffer) -> Self { db.into_buffer() }
}

/// Reads and writes target the buffer currently being drawn into.
impl AsBuffer for DoubleBuffer {
	fn size(&self) -> UVec2 { self.current_buffer().size() }
	fn allocated_rows(&self) -> u32 { self.current_buffer().allocated_rows() }
	fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.current_buffer().get(pos)
	}
	fn set(&mut self, pos: UVec2, cell: Cell) {
		self.current_buffer_mut().set(pos, cell);
	}
	fn clear(&mut self) { self.current_buffer_mut().clear(); }
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::style::VisualStyle;

	fn glyph(symbol: &str) -> Cell {
		Cell::new(symbol, VisualStyle::default(), Entity::PLACEHOLDER)
	}

	/// A changed wide-glyph trailing half is diffed as its owner glyph (one
	/// column left), never a bare placeholder whose space would clobber the
	/// glyph's right half on screen.
	#[beet_core::test]
	fn diff_redraws_wide_owner_for_changed_continuation() {
		let mut buffer = DoubleBuffer::new(UVec2::new(4, 1));
		// frame 1 on screen: "ab"
		buffer.set(UVec2::new(0, 0), glyph("a"));
		buffer.set(UVec2::new(1, 0), glyph("b"));
		buffer.swap_buffers();
		// frame 2: a wide glyph covering both columns
		buffer.set(UVec2::new(0, 0), glyph("中"));
		buffer.set(UVec2::new(1, 0), Cell {
			symbol: None,
			style: VisualStyle::default(),
			entity: Some(Entity::PLACEHOLDER),
			link: None,
		});
		let diffed: Vec<_> = buffer.diff().collect();
		// both changed cells resolve to the owner glyph at column 0
		diffed
			.iter()
			.all(|(pos, _)| *pos == UVec2::ZERO)
			.xpect_true();
		diffed
			.iter()
			.all(|(_, cell)| cell.symbol_str() == "中")
			.xpect_true();
	}

	/// A resize flags the renderer for a full screen erase, consumed once.
	#[beet_core::test]
	fn resize_flags_needs_clear() {
		let mut buffer = DoubleBuffer::new(UVec2::new(4, 1));
		buffer.take_needs_clear().xpect_false();
		buffer.resize(UVec2::new(8, 2));
		buffer.take_needs_clear().xpect_true();
		buffer.take_needs_clear().xpect_false();
	}
}
