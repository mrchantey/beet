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

	/// Iterate over all non-empty cells with their positions.
	pub fn iter_cells(&self) -> impl Iterator<Item = (UVec2, &Cell)> + '_ {
		let width = self.size.x;
		self.cells
			.iter()
			.enumerate()
			.filter_map(move |(idx, cell)| {
				cell.as_ref().map(|c| {
					let x = idx as u32 % width;
					let y = idx as u32 / width;
					(UVec2::new(x, y), c)
				})
			})
	}

	/// Write text starting at position, wrapping each character into a cell.
	pub fn write_text(
		&mut self,
		pos: UVec2,
		text: &str,
		style: VisualStyle,
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
					let ansi_style = visual_style_to_ansi(&cell.style);
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
	pub style: VisualStyle,
	pub entity: Entity,
}

impl Cell {
	pub fn new(
		symbol: impl Into<SmolStr>,
		style: impl Into<VisualStyle>,
		entity: Entity,
	) -> Self {
		Self {
			symbol: symbol.into(),
			style: style.into(),
			entity,
		}
	}
}

// ── ANSI conversions ──────────────────────────────────────────────────────────

#[cfg(feature = "ansi_paint")]
fn color_to_ansi(color: Color) -> nu_ansi_term::Color {
	let s = color.to_srgba_u8();
	nu_ansi_term::Color::Rgb(s.red, s.green, s.blue)
}

/// Convert a [`VisualStyle`] to a nu_ansi_term style for buffer rendering.
#[cfg(feature = "ansi_paint")]
fn visual_style_to_ansi(style: &VisualStyle) -> nu_ansi_term::Style {
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

	let ts = style.text_style;
	if ts.contains(TextStyle::UNDERLINE) {
		ansi_style = ansi_style.underline();
	}
	if ts.contains(TextStyle::LINE_THROUGH) {
		ansi_style = ansi_style.strikethrough();
	}
	// TextStyle::OVERLINE: not supported in nu_ansi_term, skip

	ansi_style
}

// ── Crossterm conversions ────────────────────────────────────────────────────

/// Convert a bevy [`Color`] to a crossterm terminal color via RGB.
#[cfg(feature = "crossterm")]
fn color_to_crossterm(color: Color) -> crossterm::style::Color {
	let s = color.to_srgba_u8();
	crossterm::style::Color::Rgb {
		r: s.red,
		g: s.green,
		b: s.blue,
	}
}

#[cfg(feature = "crossterm")]
impl VisualStyle {
	/// Converts to a crossterm [`ContentStyle`](crossterm::style::ContentStyle).
	pub fn to_crossterm_content_style(&self) -> crossterm::style::ContentStyle {
		use crossterm::style::Attribute;
		let mut attributes = crossterm::style::Attributes::default();
		let s = self.text_style;
		// text weight / presentation
		if s.contains(TextStyle::BOLD) {
			attributes.set(Attribute::Bold);
		}
		if s.contains(TextStyle::ITALIC) {
			attributes.set(Attribute::Italic);
		}
		// dim is derived from foreground alpha < 50%
		if let Some(fg) = self.foreground {
			if fg.to_srgba_u8().alpha < 128 {
				attributes.set(Attribute::Dim);
			}
		}
		// underline variants: UNDERLINE + style modifier
		if s.contains(TextStyle::UNDERLINE) {
			if s.contains(TextStyle::DOUBLE) {
				attributes.set(Attribute::DoubleUnderlined);
			} else {
				// WAVY and DASH both map to basic underline (ANSI undercurl not universal)
				attributes.set(Attribute::Underlined);
			}
		}
		if s.contains(TextStyle::OVERLINE) {
			attributes.set(Attribute::OverLined);
		}
		if s.contains(TextStyle::LINE_THROUGH) {
			attributes.set(Attribute::CrossedOut);
		}
		if s.contains(TextStyle::BLINK) {
			attributes.set(Attribute::SlowBlink);
		}
		if s.contains(TextStyle::RAPID_BLINK) {
			attributes.set(Attribute::RapidBlink);
		}
		if s.contains(TextStyle::REVERSED) {
			attributes.set(Attribute::Reverse);
		}
		if s.contains(TextStyle::HIDDEN) {
			attributes.set(Attribute::Hidden);
		}
		crossterm::style::ContentStyle {
			foreground_color: self.foreground.map(color_to_crossterm),
			background_color: self.background.map(color_to_crossterm),
			underline_color: self.decoration_color.map(color_to_crossterm),
			attributes,
		}
	}
}

// ── Ratatui conversions ───────────────────────────────────────────────────────

/// Convert a bevy [`Color`] to a ratatui terminal color via RGB.
#[cfg(feature = "tui")]
fn color_to_ratatui(color: Color) -> ratatui::style::Color {
	let s = color.to_srgba_u8();
	ratatui::style::Color::Rgb(s.red, s.green, s.blue)
}

#[cfg(feature = "tui")]
impl VisualStyle {
	/// Converts to a ratatui [`Style`](ratatui::style::Style).
	pub fn to_ratatui_style(&self) -> ratatui::style::Style {
		let mut modifier = ratatui::style::Modifier::empty();
		let s = self.text_style;
		if s.contains(TextStyle::BOLD) {
			modifier |= ratatui::style::Modifier::BOLD;
		}
		if s.contains(TextStyle::ITALIC) {
			modifier |= ratatui::style::Modifier::ITALIC;
		}
		// dim derived from foreground alpha
		if let Some(fg) = self.foreground {
			if fg.to_srgba_u8().alpha < 128 {
				modifier |= ratatui::style::Modifier::DIM;
			}
		}
		if s.contains(TextStyle::BLINK) {
			modifier |= ratatui::style::Modifier::SLOW_BLINK;
		}
		if s.contains(TextStyle::RAPID_BLINK) {
			modifier |= ratatui::style::Modifier::RAPID_BLINK;
		}
		if s.contains(TextStyle::REVERSED) {
			modifier |= ratatui::style::Modifier::REVERSED;
		}
		if s.contains(TextStyle::HIDDEN) {
			modifier |= ratatui::style::Modifier::HIDDEN;
		}
		if s.contains(TextStyle::UNDERLINE) {
			modifier |= ratatui::style::Modifier::UNDERLINED;
		}
		if s.contains(TextStyle::LINE_THROUGH) {
			modifier |= ratatui::style::Modifier::CROSSED_OUT;
		}
		// OVERLINE has no ratatui Modifier equivalent
		ratatui::style::Style {
			fg: self.foreground.map(color_to_ratatui),
			bg: self.background.map(color_to_ratatui),
			underline_color: self.decoration_color.map(color_to_ratatui),
			add_modifier: modifier,
			sub_modifier: ratatui::style::Modifier::empty(),
		}
	}
}

#[cfg(feature = "tui")]
impl Cell {
	/// Converts to a ratatui [`Cell`](ratatui::buffer::Cell).
	pub fn to_ratatui_cell(&self) -> ratatui::buffer::Cell {
		let mut cell = ratatui::buffer::Cell::default();
		cell.set_symbol(self.symbol.as_str());
		cell.set_style(self.style.to_ratatui_style());
		cell
	}
}
