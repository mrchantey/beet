use super::*;

// ── Text ──────────────────────────────────────────────────────────────────────

pub struct TextWidget {
	pub content: String,
	pub align: TextAlign,
}

impl TextWidget {
	pub fn new(content: impl Into<String>) -> Self {
		Self {
			content: content.into(),
			align: TextAlign::Left,
		}
	}
	pub fn align(mut self, a: TextAlign) -> Self {
		self.align = a;
		self
	}
}

impl Widget for TextWidget {
	fn measure(&self, available: Size) -> Size {
		let lines = word_wrap(&self.content, available.w);
		Size {
			w: lines.iter().map(|l| display_width(l)).max().unwrap_or(0) as u16,
			h: lines.len() as u16,
		}
	}

	fn layout(&self, rect: Rect, out: &mut Vec<Cell>) {
		for (i, line) in word_wrap(&self.content, rect.w).iter().enumerate() {
			let y = rect.y + i as u16;
			if y >= rect.y + rect.h {
				break;
			}
			out.push(Cell {
				x: rect.x,
				y,
				text: align_line(line, rect.w, self.align),
			});
		}
	}
}



// ── Word wrap ─────────────────────────────────────────────────────────────────

fn word_wrap(text: &str, max_w: u16) -> Vec<String> {
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

fn align_line(line: &str, width: u16, align: TextAlign) -> String {
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
