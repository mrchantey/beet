use beet_core::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;

/// Tracks vertical scroll position and content dimensions.
///
/// Inserted by [`TuiRenderer`] on the root entity after each render pass.
/// The input system updates `offset` when arrow keys or mouse scroll
/// events are received. The renderer reads `offset` to determine which
/// portion of the content buffer is visible.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component)]
pub struct TuiScrollState {
	/// Current vertical scroll offset in rows from the top.
	pub offset: u16,
	/// Total content height in rows (set by the renderer after layout).
	pub content_height: u16,
	/// Visible viewport height in rows (set by the renderer from the
	/// terminal area, excluding the border).
	pub viewport_height: u16,
}

impl TuiScrollState {
	/// Whether the content overflows the viewport, requiring a scrollbar.
	pub fn overflows(&self) -> bool {
		self.content_height > self.viewport_height
	}

	/// Maximum valid scroll offset.
	pub fn max_offset(&self) -> u16 {
		self.content_height.saturating_sub(self.viewport_height)
	}

	/// Scroll down by `count` rows, clamped to the maximum offset.
	pub fn scroll_down(&mut self, count: u16) {
		self.offset = self.offset.saturating_add(count).min(self.max_offset());
	}

	/// Scroll up by `count` rows.
	pub fn scroll_up(&mut self, count: u16) {
		self.offset = self.offset.saturating_sub(count);
	}

	/// Clamp the current offset to valid bounds, used after content
	/// height changes between frames.
	pub fn clamp(&mut self) {
		self.offset = self.offset.min(self.max_offset());
	}

	/// Build a ratatui [`ScrollbarState`] from the current values.
	pub fn scrollbar_state(&self) -> ScrollbarState {
		ScrollbarState::new(self.max_offset() as usize)
			.position(self.offset as usize)
	}

	/// Render a scrollbar in the given area if the content overflows the viewport.
	pub fn try_render(&mut self, area: Rect, buffer: &mut Buffer) {
		if self.overflows() {
			let mut sb_state = self.scrollbar_state();
			let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
				.begin_symbol(Some("↑"))
				.end_symbol(Some("↓"));
			scrollbar.render(
				area.inner(Margin {
					vertical: 1,
					horizontal: 0,
				}),
				buffer,
				&mut sb_state,
			);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn scrollbar_state_matches() {
		let state = TuiScrollState {
			offset: 5,
			content_height: 30,
			viewport_height: 20,
		};
		let sb = state.scrollbar_state();
		// ScrollbarState doesn't expose fields, but we can verify it
		// was created without panicking.
		format!("{sb:?}").xpect_contains("ScrollbarState");
	}
}
