//! Terminal hyperlink widget using OSC 8 escape sequences.
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
use ratatui::prelude::Widget;
use ratatui::text::Text;

/// A terminal hyperlink using OSC 8 escape sequences.
///
/// Renders clickable links in supported terminals.
#[derive(Clone, Default)]
pub struct Hyperlink {
	text: Text<'static>,
	url: String,
}

impl Hyperlink {
	/// Create a hyperlink with the given display text and URL.
	pub fn new(text: impl Into<Text<'static>>, url: impl Into<String>) -> Self {
		Self {
			text: text.into(),
			url: url.into(),
		}
	}
}

impl Widget for &Hyperlink {
	fn render(self, area: Rect, buffer: &mut Buffer) {
		(&self.text).render(area, buffer);

		// OSC 8 hyperlink rendering in 2-char chunks to work around
		// ratatui's ANSI width calculation bug.
		let chars: Vec<char> = self.text.to_string().chars().collect();
		for (idx, chunk) in chars.chunks(2).enumerate() {
			let text: String = chunk.iter().collect();
			let hyperlink =
				format!("\x1B]8;;{}\x07{}\x1B]8;;\x07", self.url, text);
			let col = area.x + idx as u16 * 2;
			if col < area.x + area.width {
				buffer[(col, area.y)].set_symbol(hyperlink.as_str());
			}
		}
	}
}

impl Widget for Hyperlink {
	fn render(self, area: Rect, buffer: &mut Buffer) {
		(&self).render(area, buffer);
	}
}
