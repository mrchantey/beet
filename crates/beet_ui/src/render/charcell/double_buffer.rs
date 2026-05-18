use super::*;
use beet_core::prelude::*;
use bevy::math::UVec2;
use core::ops::Deref;
use core::ops::DerefMut;

/// Double-buffered terminal buffer.
///
/// Holds two [`Buffer`]s: one for the current frame being drawn,
/// one for the previous frame (used for diffing). Only changed cells
/// are written to the terminal on each render.
#[derive(Component)]
pub struct DoubleBuffer {
	buffers: [Buffer; 2],
	current: usize,
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
		}
	}

	pub fn from_buffer(buffer: Buffer) -> Self {
		Self {
			buffers: [buffer.clone(), buffer],
			current: 0,
		}
	}


	pub fn into_buffer(self) -> Buffer {
		self.buffers.into_iter().nth(self.current).unwrap()
	}

	/// The buffer currently being drawn into.
	pub fn current_buffer(&self) -> &Buffer { &self.buffers[self.current] }

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
				if changed {
					Some((UVec2::new(i as u32 % width, i as u32 / width), cell))
				} else {
					None
				}
			})
	}
}


impl From<Buffer> for DoubleBuffer {
	fn from(buffer: Buffer) -> Self { Self::from_buffer(buffer) }
}
impl From<DoubleBuffer> for Buffer {
	fn from(db: DoubleBuffer) -> Self { db.into_buffer() }
}
