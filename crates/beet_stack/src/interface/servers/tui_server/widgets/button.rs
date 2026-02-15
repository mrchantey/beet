//! A simple TUI button widget for invoking tools.
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::Widget;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;

/// A simple button widget for invoking tools with `()` input.
#[derive(Clone)]
pub struct Button {
	label: Line<'static>,
	selected: bool,
}

impl Button {
	/// Create a button with the given label.
	pub fn new(label: impl Into<Line<'static>>) -> Self {
		Self {
			label: label.into(),
			selected: false,
		}
	}

	/// Mark this button as selected.
	pub fn selected(mut self, selected: bool) -> Self {
		self.selected = selected;
		self
	}
}

impl Widget for Button {
	fn render(self, area: Rect, buf: &mut Buffer) {
		let (bg, fg) = if self.selected {
			(Color::White, Color::Black)
		} else {
			(Color::DarkGray, Color::White)
		};
		buf.set_style(area, Style::new().bg(bg).fg(fg));

		// Top highlight
		if area.height > 2 {
			buf.set_string(
				area.x,
				area.y,
				"\u{2594}".repeat(area.width as usize),
				Style::new().fg(Color::Gray).bg(bg),
			);
		}
		// Bottom shadow
		if area.height > 1 {
			buf.set_string(
				area.x,
				area.y + area.height - 1,
				"\u{2581}".repeat(area.width as usize),
				Style::new().fg(Color::Black).bg(bg),
			);
		}
		// Label centered
		buf.set_line(
			area.x + (area.width.saturating_sub(self.label.width() as u16)) / 2,
			area.y + (area.height.saturating_sub(1)) / 2,
			&self.label,
			area.width,
		);
	}
}
