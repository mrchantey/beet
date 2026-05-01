use super::*;
use bevy::math::URect;
use bevy::math::UVec2;

// ── Text ──────────────────────────────────────────────────────────────────────

pub struct TextWidget {
	pub content: String,
	pub layout_style: LayoutStyle,
	pub align: TextAlign,
}

impl TextWidget {
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
			layout_style: LayoutStyle::default(),
			align: TextAlign::Left,
		}
	}
	pub fn align(mut self, a: TextAlign) -> Self {
		self.align = a;
		self
	}
	pub fn flex_grow(mut self, grow: u32) -> Self {
		self.layout_style.flex_grow = grow;
		self
	}
}

impl Widget for TextWidget {
	fn layout_style(&self) -> &LayoutStyle { &self.layout_style }

	fn measure(&self, available: UVec2) -> UVec2 {
		let lines = word_wrap(&self.content, available.x);
		UVec2::new(
			lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u32,
			lines.len() as u32,
		)
	}

	fn layout(&self, buffer: &mut Buffer, rect: URect) {
		let lines = word_wrap(&self.content, rect.width());
		for (i, line) in lines.iter().enumerate() {
			let y = rect.min.y + i as u32;
			if y >= rect.max.y {
				break;
			}
			let aligned = align_line(line, rect.width(), self.align);
			buffer.write_text(
				UVec2::new(rect.min.x, y),
				&aligned,
				VisualStyle::default(),
			);
		}
	}
}

// ── Word wrap ─────────────────────────────────────────────────────────────────

fn word_wrap(text: &str, max_w: u32) -> Vec<String> {
	let max_w = max_w as usize;
	if max_w == 0 {
		return vec![];
	}
	let mut lines = Vec::new();

	for para in text.split('\n') {
		let mut current = String::new();
		for word in para.split_whitespace() {
			if current.is_empty() {
				// Hard-break words longer than the column
				let mut w = word;
				while display_width(w) > max_w {
					lines.push(w[..max_w].to_string());
					w = &w[max_w..];
				}
				current = w.to_string();
			} else if current.len() + 1 + word.len() <= max_w {
				current.push(' ');
				current.push_str(word);
			} else {
				lines.push(std::mem::take(&mut current));
				current = word.to_string();
			}
		}
		lines.push(current);
	}
	lines
}

/// Counts visible characters, skipping ANSI escape sequences.
pub fn display_width(s: &str) -> usize {
	let mut w = 0;
	let mut in_esc = false;
	for ch in s.chars() {
		match ch {
			'\x1b' => in_esc = true,
			'm' if in_esc => in_esc = false,
			_ if !in_esc => w += 1,
			_ => {}
		}
	}
	w
}

fn align_line(line: &str, width: u32, align: TextAlign) -> String {
	let w = width as usize;
	let len = display_width(line);
	if len >= w {
		return line.chars().take(w).collect();
	}
	let pad = w - len;
	match align {
		TextAlign::Left => format!("{line:<w$}"),
		TextAlign::Right => format!("{line:>w$}"),
		TextAlign::Center => {
			let l = pad / 2;
			format!("{}{line}{}", " ".repeat(l), " ".repeat(pad - l))
		}
	}
}
