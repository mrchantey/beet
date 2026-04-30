use super::*;


// ── Renderer ──────────────────────────────────────────────────────────────────

pub fn render(cells: &[Cell], width: u16, height: u16) -> String {
	let mut buf: Vec<Vec<char>> =
		vec![vec![' '; width as usize]; height as usize];
	for cell in cells {
		let row = cell.y as usize;
		if row >= buf.len() {
			continue;
		}
		for (i, ch) in cell.text.chars().enumerate() {
			let col = cell.x as usize + i;
			if col < buf[row].len() {
				buf[row][col] = ch;
			}
		}
	}
	buf.iter()
		.map(|r| r.iter().collect::<String>())
		.collect::<Vec<_>>()
		.join("\n")
}


// ── Widget trait ──────────────────────────────────────────────────────────────

pub trait Widget {
	fn layout_style(&self) -> &LayoutStyle;

	/// Pass 1 (bottom-up): given available space as a hint, return desired size.
	fn measure(&self, available: Size) -> Size;

	/// Pass 2 (top-down): given the assigned rect, emit render cells.
	fn layout(&self, rect: Rect, out: &mut Vec<Cell>);
}


pub struct Button {
	pub label: String,
	pub layout_style: LayoutStyle,
}

impl Widget for Button {
	fn layout_style(&self) -> &LayoutStyle { &self.layout_style }
	fn measure(&self, _available: Size) -> Size {
		Size {
			w: self.label.len() as u16 + 4, // "[ label ]"
			h: 3,                           // top border, content, bottom border
		}
	}
	fn layout(&self, rect: Rect, out: &mut Vec<Cell>) {
		let iw = rect.w.saturating_sub(2) as usize;
		out.push(Cell {
			x: rect.x,
			y: rect.y,
			text: format!("┌{}┐", "─".repeat(iw)),
		});
		out.push(Cell {
			x: rect.x,
			y: rect.y + 1,
			text: format!("│{:^iw$}│", self.label),
		});
		out.push(Cell {
			x: rect.x,
			y: rect.y + 2,
			text: format!("└{}┘", "─".repeat(iw)),
		});
	}
}
