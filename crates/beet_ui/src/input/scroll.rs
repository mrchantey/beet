//! Renderer-agnostic scroll model: the per-container scroll offset and the
//! state math that clamps it, detects overflow, sizes a scrollbar thumb, and
//! maps between screen and content space.
//!
//! This is the resolution-independent half of scrolling (the DOM/retained-mode
//! model, like `bevy_ui`): layout stays unscrolled, and scrolling is a paint +
//! hit-test translation. The charcell side reserves the gutter, translates the
//! paint, and draws the bar; everything here is pure data and arithmetic, reused
//! by the future native renderer.

use beet_core::prelude::*;
use bevy::math::IVec2;
use bevy::math::UVec2;

/// The scroll offset of an overflow container, in cells, persisted across frames.
///
/// `offset` is how far the content has scrolled: a positive `y` moves content up
/// (later rows come into view), a positive `x` moves it left. The paint and
/// hit-test passes translate descendants by `-offset`. Auto-inserted on a node
/// that becomes a scroll container.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Component, Get, Set,
)]
#[reflect(Component)]
pub struct ScrollPosition {
	/// Scrolled distance in cells from the content origin, per axis.
	pub offset: IVec2,
}

impl ScrollPosition {
	/// A scroll position at the given cell offset.
	pub fn new(offset: IVec2) -> Self { Self { offset } }

	/// Clamp the offset into the valid range for `state`, ie after a content or
	/// scrollport size change. Returns whether the offset changed.
	pub fn clamp_to(&mut self, state: &ScrollState) -> bool {
		let clamped = state.clamp_offset(self.offset);
		let changed = clamped != self.offset;
		self.offset = clamped;
		changed
	}

	/// Scroll by `delta` cells, clamped to `state`. Returns whether it changed.
	pub fn scroll_by(&mut self, delta: IVec2, state: &ScrollState) -> bool {
		let next = state.clamp_offset(self.offset + delta);
		let changed = next != self.offset;
		self.offset = next;
		changed
	}
}

/// A snapshot of one container's scroll geometry: the agnostic scroll-state math
/// (clamp / overflow / thumb), per axis.
///
/// Built per frame from the container's content size (its unconstrained
/// [`IntrinsicSize`](crate::prelude::IntrinsicSize)) and its scrollport (the
/// laid-out content rect minus the reserved scrollbar gutter). The renderer asks
/// it for the clamped offset, overflow flags, and thumb geometry; nothing here
/// touches cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollState {
	/// Full scrollable content size in cells (unconstrained).
	pub content: UVec2,
	/// Visible scrollport size in cells (content rect minus gutter).
	pub scrollport: UVec2,
}

impl ScrollState {
	/// Build from a content size and a scrollport size.
	pub fn new(content: UVec2, scrollport: UVec2) -> Self {
		Self { content, scrollport }
	}

	/// The maximum scroll offset per axis: how far the content can move before
	/// its far edge reaches the scrollport edge. Zero on an axis that fits.
	pub fn max_offset(&self) -> IVec2 {
		IVec2::new(
			self.content.x.saturating_sub(self.scrollport.x) as i32,
			self.content.y.saturating_sub(self.scrollport.y) as i32,
		)
	}

	/// Clamp `offset` into `[0, max_offset]` on both axes.
	pub fn clamp_offset(&self, offset: IVec2) -> IVec2 {
		offset.clamp(IVec2::ZERO, self.max_offset())
	}

	/// Whether the content overflows the scrollport horizontally.
	pub fn overflows_x(&self) -> bool { self.content.x > self.scrollport.x }

	/// Whether the content overflows the scrollport vertically.
	pub fn overflows_y(&self) -> bool { self.content.y > self.scrollport.y }

	/// Vertical thumb `(start_row, len)` within a `track_len`-row track, sized to
	/// the visible fraction and positioned by the offset. `None` when not
	/// overflowing vertically.
	pub fn thumb_y(&self, offset: IVec2, track_len: u32) -> Option<(u32, u32)> {
		thumb(self.content.y, self.scrollport.y, offset.y, track_len)
	}

	/// Horizontal thumb `(start_col, len)`, analogous to [`thumb_y`](Self::thumb_y).
	pub fn thumb_x(&self, offset: IVec2, track_len: u32) -> Option<(u32, u32)> {
		thumb(self.content.x, self.scrollport.x, offset.x, track_len)
	}
}

/// One axis of thumb geometry: `(start, len)` in track cells, or `None` when the
/// content fits (no thumb). The thumb length is the visible fraction of the
/// track (at least 1 cell), positioned proportionally to the scroll progress.
fn thumb(
	content: u32,
	scrollport: u32,
	offset: i32,
	track_len: u32,
) -> Option<(u32, u32)> {
	if content <= scrollport || track_len == 0 || scrollport == 0 {
		return None;
	}
	let track = track_len as f32;
	let len = (track * scrollport as f32 / content as f32)
		.round()
		.max(1.)
		.min(track) as u32;
	let max_offset = content.saturating_sub(scrollport).max(1) as f32;
	let progress = (offset.max(0) as f32 / max_offset).clamp(0., 1.);
	// the thumb travels the leftover track (track_len - len), pinned to the
	// bottom/right at full scroll so the end is reachable.
	let travel = track_len.saturating_sub(len) as f32;
	let start = (progress * travel).round() as u32;
	Some((start.min(track_len - len), len))
}

#[cfg(test)]
mod test {
	use super::*;

	fn state(content_y: u32, port_y: u32) -> ScrollState {
		ScrollState::new(UVec2::new(10, content_y), UVec2::new(10, port_y))
	}

	#[beet_core::test]
	fn max_offset_and_overflow() {
		let s = state(30, 20);
		s.max_offset().y.xpect_eq(10);
		s.overflows_y().xpect_true();
		state(10, 20).overflows_y().xpect_false();
		state(10, 20).max_offset().y.xpect_eq(0);
	}

	#[beet_core::test]
	fn clamp_holds_bounds() {
		let s = state(30, 20);
		// past the max clamps down, below zero clamps up
		s.clamp_offset(IVec2::new(0, 99)).y.xpect_eq(10);
		s.clamp_offset(IVec2::new(0, -5)).y.xpect_eq(0);
		s.clamp_offset(IVec2::new(0, 4)).y.xpect_eq(4);
	}

	#[beet_core::test]
	fn position_scroll_and_reclamp() {
		let s = state(30, 20);
		let mut pos = ScrollPosition::default();
		pos.scroll_by(IVec2::new(0, 4), &s).xpect_true();
		pos.offset.y.xpect_eq(4);
		// scrolling past the end clamps and reports the change up to the bound
		pos.scroll_by(IVec2::new(0, 100), &s);
		pos.offset.y.xpect_eq(10);
		// content shrinks: re-clamp pulls the offset back into range
		let shrunk = state(22, 20);
		pos.clamp_to(&shrunk).xpect_true();
		pos.offset.y.xpect_eq(2);
	}

	#[beet_core::test]
	fn thumb_sizes_to_visible_fraction() {
		// half the content visible -> thumb spans half the track, at the top
		let s = state(40, 20);
		let (start, len) = s.thumb_y(IVec2::ZERO, 20).unwrap();
		start.xpect_eq(0);
		len.xpect_eq(10);
		// scrolled to the end -> thumb pinned to the bottom of the track
		let (start, len) = s.thumb_y(s.max_offset(), 20).unwrap();
		(start + len).xpect_eq(20);
		// no overflow -> no thumb
		state(10, 20).thumb_y(IVec2::ZERO, 20).xpect_none();
	}
}
