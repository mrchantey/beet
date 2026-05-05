use crate::style::TextStyle;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// A rectangular buffer of cells, indexed by position.
#[derive(Clone)]
pub struct Buffer {
	size: UVec2,
	cells: Vec<Option<Cell>>,
}

impl Buffer {
	pub fn new(size: UVec2) -> Self {
		let len = (size.x * size.y) as usize;
		Self {
			size,
			cells: alloc::vec::from_elem(None, len),
		}
	}
	pub fn clear(&mut self) {
		for cell in &mut self.cells {
			*cell = None;
		}
	}

	pub fn size(&self) -> UVec2 { self.size }

	/// Convert position to buffer index.
	fn index(&self, pos: UVec2) -> Option<usize> {
		if pos.x >= self.size.x || pos.y >= self.size.y {
			return None;
		}
		Some((pos.y * self.size.x + pos.x) as usize)
	}

	/// Set a cell at the given position.
	pub fn set(&mut self, pos: UVec2, cell: Cell) {
		if let Some(idx) = self.index(pos) {
			self.cells[idx] = Some(cell);
		}
	}

	/// Get a cell at the given position.
	pub fn get(&self, pos: UVec2) -> Option<&Cell> {
		self.index(pos).and_then(|idx| self.cells[idx].as_ref())
	}

	/// Write text starting at position, wrapping each character into a cell.
	pub fn write_text(
		&mut self,
		pos: UVec2,
		text: &str,
		style: impl Clone + Into<CharStyle>,
		entity: Entity,
	) {
		for (i, ch) in text.chars().enumerate() {
			let cell_pos = UVec2::new(pos.x + i as u32, pos.y);
			if cell_pos.x >= self.size.x {
				break;
			}
			self.set(
				cell_pos,
				Cell::new(ch.to_string(), style.clone(), entity),
			);
		}
	}

	pub fn render(&self) -> String {
		cfg_if! {
			if #[cfg(feature = "ansi_paint")] {
				self.render_ansi()
			} else {
				self.render_plain()
			}
		}
	}

	/// Render the buffer to a string (plain text, no styling).
	pub fn render_plain(&self) -> String {
		let width = self.size.x as usize;
		let height = self.size.y as usize;
		let mut result = String::with_capacity(self.cells.len());

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				if let Some(cell) = &self.cells[idx] {
					result.push_str(&cell.symbol);
				} else {
					result.push(' ');
				}
			}
			if y < height - 1 {
				result.push('\n');
			}
		}
		result
	}

	/// Render the buffer to a string with ANSI styling.
	#[cfg(feature = "ansi_paint")]
	pub fn render_ansi(&self) -> String {
		let width = self.size.x as usize;
		let height = self.size.y as usize;
		let mut result = String::with_capacity(self.cells.len());

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				if let Some(cell) = &self.cells[idx] {
					let ansi_style = char_style_to_ansi(&cell.style);
					result.push_str(
						&ansi_style.paint(cell.symbol.as_str()).to_string(),
					);
				} else {
					result.push(' ');
				}
			}
			if y < height - 1 {
				result.push('\n');
			}
		}
		result
	}
}

/// A single terminal cell with text and styling.
#[derive(Debug, Clone, SetWith)]
pub struct Cell {
	pub symbol: SmolStr,
	#[set_with(into)]
	pub style: CharStyle,
	pub entity: Entity,
}

impl Cell {
	pub fn new(
		symbol: impl Into<SmolStr>,
		style: impl Into<CharStyle>,
		entity: Entity,
	) -> Self {
		Self {
			symbol: symbol.into(),
			style: style.into(),
			entity,
		}
	}
}

#[derive(Debug, Default, Clone, PartialEq, SetWith)]
pub struct CharStyle {
	/// In ansi renderers an alpha channel of <50% will apply the `dim` attribute
	pub foreground: Option<Color>,
	pub background: Option<Color>,
	pub decoration_color: Option<Color>,
	pub text_style: Vec<TextStyle>,
}

impl From<VisualStyle> for CharStyle {
	fn from(style: VisualStyle) -> Self {
		Self {
			foreground: style.foreground,
			background: style.background,
			decoration_color: style.decoration_color,
			text_style: style.text_style,
		}
	}
}

#[cfg(feature = "ansi_paint")]
fn color_to_ansi(color: Color) -> nu_ansi_term::Color {
	let s = color.to_srgba_u8();
	nu_ansi_term::Color::Rgb(s.red, s.green, s.blue)
}

#[cfg(feature = "ansi_paint")]
fn char_style_to_ansi(style: &CharStyle) -> nu_ansi_term::Style {
	let mut ansi_style = nu_ansi_term::Style::new();

	if let Some(color) = style.foreground {
		let s = color.to_srgba_u8();
		ansi_style = ansi_style.fg(color_to_ansi(color));
		// alpha < 50% maps to the terminal `dim` attribute
		if s.alpha < 128 {
			ansi_style = ansi_style.dimmed();
		}
	}

	if let Some(color) = style.background {
		ansi_style = ansi_style.on(color_to_ansi(color));
	}

	for decoration in &style.text_style {
		match decoration {
			TextStyle::Underline
			| TextStyle::UnderlineDouble
			| TextStyle::UnderlineWavy
			| TextStyle::UnderlinDash => {
				ansi_style = ansi_style.underline();
			}
			TextStyle::Overline => {
				// not supported in ANSI terminals, skip
			}
			TextStyle::LineThrough => {
				ansi_style = ansi_style.strikethrough();
			}
		}
	}

	ansi_style
}
