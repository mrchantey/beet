//! Scrollbar rendering helper for the TUI.
//!
//! Wraps ratatui's [`Scrollbar`] and [`ScrollbarState`] into a
//! simple helper that handles layout splitting and conditional
//! visibility.
use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::widgets::Scrollbar as RatScrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;

/// Per-card scroll position, inserted alongside [`CurrentCard`].
#[derive(Default, Component)]
pub struct TuiScrollState {
	/// Current vertical scroll offset (in lines).
	pub offset: u16,
}



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


/// Handle scroll input events for the TUI.
pub(crate) fn handle_scroll_input(
	mut messages: MessageReader<bevy::input::keyboard::KeyboardInput>,
	mut scroll_query: Query<&mut TuiScrollState, With<CurrentCard>>,
) {
	use bevy::input::keyboard::Key;
	for message in messages.read() {
		let Ok(mut scroll) = scroll_query.single_mut() else {
			return;
		};
		match &message.logical_key {
			Key::Character(val) if val == "j" => {
				scroll.offset = scroll.offset.saturating_add(1);
			}
			Key::Character(val) if val == "k" => {
				scroll.offset = scroll.offset.saturating_sub(1);
			}
			Key::ArrowDown => {
				scroll.offset = scroll.offset.saturating_add(1);
			}
			Key::ArrowUp => {
				scroll.offset = scroll.offset.saturating_sub(1);
			}
			_ => {}
		}
	}
}
