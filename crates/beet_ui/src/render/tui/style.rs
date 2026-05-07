use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;


/// Horizontal justification for block-level content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
	#[default]
	Start,
	Center,
	End,
}

/// Style descriptor for TUI rendering.
///
/// Wraps a ratatui [`Style`] with layout metadata so the
/// [`StyleMap`] can drive both visual appearance and block-level
/// spacing without hand-rolled config structs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiStyle {
	/// The ratatui visual style (colors, modifiers).
	pub style: Style,
	/// Blank lines to emit before this element.
	pub lines_before: u16,
	/// Blank lines to emit after this element.
	pub lines_after: u16,
	/// Horizontal justification for block content.
	pub justify: Justify,
}

impl Default for TuiStyle {
	fn default() -> Self {
		Self {
			style: Style::default(),
			lines_before: 0,
			lines_after: 0,
			justify: Justify::Start,
		}
	}
}

impl TuiStyle {
	pub fn new(style: Style) -> Self {
		Self {
			style,
			..Default::default()
		}
	}

	pub fn with_lines_before(mut self, lines: u16) -> Self {
		self.lines_before = lines;
		self
	}

	pub fn with_lines_after(mut self, lines: u16) -> Self {
		self.lines_after = lines;
		self
	}

	pub fn with_justify(mut self, justify: Justify) -> Self {
		self.justify = justify;
		self
	}
}

/// Build the default element → [`TuiStyle`] mapping used by [`TuiRenderer`].
pub fn default_tui_style_map() -> StyleMap<TuiStyle> {
	StyleMap::new(TuiStyle::default(), vec![
		(
			"h1",
			TuiStyle::new(Style::new().bold().fg(Color::Green))
				.with_lines_before(1)
				.with_lines_after(1)
				.with_justify(Justify::Center),
		),
		("h2", TuiStyle::new(Style::new().bold().fg(Color::Cyan))),
		("h3", TuiStyle::new(Style::new().bold())),
		("h4", TuiStyle::new(Style::new().bold())),
		("h5", TuiStyle::new(Style::new().bold())),
		("h6", TuiStyle::new(Style::new().bold())),
		("p", TuiStyle::default()),
		("div", TuiStyle::default()),
		("section", TuiStyle::default()),
		("article", TuiStyle::default()),
		("nav", TuiStyle::default()),
		("header", TuiStyle::default()),
		("footer", TuiStyle::default()),
		("main", TuiStyle::default()),
		(
			"blockquote",
			TuiStyle::new(Style::new().italic())
				.with_lines_before(1)
				.with_lines_after(1),
		),
		(
			"aside",
			TuiStyle::new(Style::new().italic())
				.with_lines_before(1)
				.with_lines_after(1),
		),
		("pre", TuiStyle::new(Style::new().bg(Color::DarkGray))),
		("code", TuiStyle::new(Style::new().bg(Color::DarkGray))),
		("strong", TuiStyle::new(Style::new().bold())),
		("em", TuiStyle::new(Style::new().italic())),
		(
			"del",
			TuiStyle::new(Style::new().add_modifier(Modifier::CROSSED_OUT)),
		),
		(
			"a",
			TuiStyle::new(
				Style::new()
					.fg(Color::Cyan)
					.add_modifier(Modifier::UNDERLINED),
			),
		),
		("hr", TuiStyle::new(Style::new().fg(Color::DarkGray))),
		(
			"img",
			TuiStyle::new(Style::new().fg(Color::DarkGray).italic()),
		),
		("button", TuiStyle::new(Style::new().fg(Color::Cyan))),
		("li", TuiStyle::default()),
		("thead", TuiStyle::new(Style::new().bold())),
	])
}
