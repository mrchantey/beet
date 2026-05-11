use crate::style::*;
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
		let mut out: Vec<u8> = Vec::with_capacity(self.cells.len() * 8);
		let mut prev_style: Option<VisualStyle> = None;

		for y in 0..height {
			for x in 0..width {
				let idx = y * width + x;
				if let Some(cell) = &self.cells[idx] {
					write_visual_style(
						&mut out,
						&cell.style,
						prev_style.as_ref(),
					);
					out.extend_from_slice(cell.symbol.as_bytes());
					prev_style = Some(cell.style.clone());
				} else {
					if prev_style.is_some() {
						out.extend_from_slice(b"\x1b[0m");
						prev_style = None;
					}
					out.push(b' ');
				}
			}
			if y < height - 1 {
				out.push(b'\n');
			}
		}
		if prev_style.is_some() {
			out.extend_from_slice(b"\x1b[0m");
		}
		String::from_utf8_lossy(&out).into_owned()
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
	/// Two cells are visually equal if their symbol and style match.
	/// The entity is disregarded
	pub fn visual_eq(&self, other: &Self) -> bool {
		self.symbol == other.symbol && self.style == other.style
	}
}

// ── ANSI conversions ──────────────────────────────────────────────────────────

/// Write ANSI escape sequences for `style`, diffing against `prev`.
#[cfg(feature = "ansi_paint")]
fn write_visual_style(
	out: &mut Vec<u8>,
	style: &VisualStyle,
	prev: Option<&VisualStyle>,
) {
	use std::io::Write;

	let is_dim = style.dim_foreground();
	let prev_dim = prev.map_or(false, |p| p.dim_foreground());

	// check if the current style has any active attributes
	let style_is_active = is_dim
		|| style.foreground.is_some()
		|| style.background.is_some()
		|| style.text_style != TextStyle::empty()
		|| style.decoration_line.underline
		|| style.decoration_line.overline
		|| style.decoration_line.line_through;

	// check if prev had any active attributes (needed to know if we must reset)
	let prev_is_active = prev.map_or(false, |p| {
		p.dim_foreground()
			|| p.foreground.is_some()
			|| p.background.is_some()
			|| p.text_style != TextStyle::empty()
			|| p.decoration_line.underline
			|| p.decoration_line.overline
			|| p.decoration_line.line_through
	});

	// skip entirely if neither current nor prev has any styling
	if !style_is_active && !prev_is_active {
		return;
	}

	let text_changed = prev.map_or(true, |p| p.text_style != style.text_style)
		|| is_dim != prev_dim;

	// reset and rewrite all attributes when text style changes
	if text_changed {
		out.extend_from_slice(b"\x1b[0m");
	}

	let effective_prev = if text_changed { None } else { prev };

	// foreground color
	if style.foreground != effective_prev.and_then(|p| p.foreground) {
		match style.foreground {
			Some(color) => {
				let c = color.to_srgba_u8();
				write!(out, "\x1b[38;2;{};{};{}m", c.red, c.green, c.blue).ok();
			}
			None => out.extend_from_slice(b"\x1b[39m"),
		}
	}

	// background color
	if style.background != effective_prev.and_then(|p| p.background) {
		match style.background {
			Some(color) => {
				let c = color.to_srgba_u8();
				write!(out, "\x1b[48;2;{};{};{}m", c.red, c.green, c.blue).ok();
			}
			None => out.extend_from_slice(b"\x1b[49m"),
		}
	}

	// decorations (always apply when text changed, otherwise check prev)
	let prev_dec = effective_prev.map(|p| &p.decoration_line);
	if style.decoration_line.underline
		&& prev_dec.map_or(true, |d| !d.underline)
	{
		out.extend_from_slice(b"\x1b[4m");
	}
	if style.decoration_line.overline && prev_dec.map_or(true, |d| !d.overline)
	{
		out.extend_from_slice(b"\x1b[53m");
	}
	if style.decoration_line.line_through
		&& prev_dec.map_or(true, |d| !d.line_through)
	{
		out.extend_from_slice(b"\x1b[9m");
	}

	// text attributes — only when text_changed
	if text_changed {
		if is_dim {
			out.extend_from_slice(b"\x1b[2m");
		}
		let ts = style.text_style;
		if ts.contains(TextStyle::BOLD) {
			out.extend_from_slice(b"\x1b[1m");
		}
		if ts.contains(TextStyle::ITALIC) {
			out.extend_from_slice(b"\x1b[3m");
		}
		if ts.contains(TextStyle::BLINK) {
			out.extend_from_slice(b"\x1b[5m");
		}
		if ts.contains(TextStyle::RAPID_BLINK) {
			out.extend_from_slice(b"\x1b[6m");
		}
		if ts.contains(TextStyle::REVERSED) {
			out.extend_from_slice(b"\x1b[7m");
		}
		if ts.contains(TextStyle::HIDDEN) {
			out.extend_from_slice(b"\x1b[8m");
		}
	}
}

// ── Ratatui conversions ───────────────────────────────────────────────────────

/// Convert a bevy [`Color`] to a ratatui terminal color via RGB.
#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
fn color_to_ratatui(color: Color) -> ratatui::style::Color {
	let s = color.to_srgba_u8();
	ratatui::style::Color::Rgb(s.red, s.green, s.blue)
}

#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
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
		if self.decoration_line.underline {
			modifier |= ratatui::style::Modifier::UNDERLINED;
		}
		if self.decoration_line.line_through {
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

#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
impl Cell {
	/// Converts to a ratatui [`Cell`](ratatui::buffer::Cell).
	pub fn to_ratatui_cell(&self) -> ratatui::buffer::Cell {
		let mut cell = ratatui::buffer::Cell::default();
		cell.set_symbol(self.symbol.as_str());
		cell.set_style(self.style.to_ratatui_style());
		cell
	}
}
