//! Scrollbar rendering helper for the TUI.
//!
//! Wraps ratatui's [`Scrollbar`] and [`ScrollbarState`] into a
//! simple helper that handles layout splitting and conditional
//! visibility.
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::widgets::Scrollbar as RatScrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;

/// Renders inner content with an optional vertical scrollbar.
///
/// The scrollbar is only shown when `content_length` exceeds the
/// available viewport height. When visible, the area is split
/// horizontally: content on the left, 1-column scrollbar on the right.
pub struct ScrollableArea<W> {
	/// The widget to render in the content area.
	pub widget: W,
	/// Total number of content lines.
	pub content_length: usize,
	/// Current scroll offset in lines.
	pub offset: usize,
}

impl<W> ScrollableArea<W> {
	/// Create a new scrollable area wrapping the given widget.
	pub fn new(widget: W, content_length: usize, offset: usize) -> Self {
		Self {
			widget,
			content_length,
			offset,
		}
	}
}

impl<W: Widget> Widget for ScrollableArea<W> {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let needs_scrollbar = self.content_length as u16 > area.height;

		if needs_scrollbar {
			let chunks =
				Layout::horizontal([Constraint::Min(1), Constraint::Length(1)])
					.split(area);

			self.widget.render(chunks[0], buf);

			let mut state =
				ScrollbarState::new(self.content_length).position(self.offset);
			RatScrollbar::new(ScrollbarOrientation::VerticalRight)
				.begin_symbol(None)
				.end_symbol(None)
				.render(chunks[1], buf, &mut state);
		} else {
			self.widget.render(area, buf);
		}
	}
}
