use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Size {
	pub w: u16,
	pub h: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
	pub x: u16,
	pub y: u16,
	pub w: u16,
	pub h: u16,
}


impl Rect {
	pub fn size(self) -> Size {
		Size {
			w: self.w,
			h: self.h,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TextAlign {
	#[default]
	Left,
	Center,
	Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
	Row,
	Col,
}

/// Where children sit on the cross axis of their flex line.
/// Row container → cross axis is vertical.
/// Col container → cross axis is horizontal.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AlignItems {
	#[default]
	Start,
	Center,
	End,
	Stretch, // expand child to fill the line's cross size
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FlexWrap {
	#[default]
	NoWrap,
	Wrap,
}

#[derive(Debug, Clone)]
pub struct Cell {
	pub x: u16,
	pub y: u16,
	pub text: String,
}

// ── FlexChild ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct LayoutStyle {
	pub flex_grow: Option<u16>,
}


pub struct FlexBox {
	pub direction: Direction,
	pub layout_style: LayoutStyle,
	pub wrap: FlexWrap,
	pub align_items: AlignItems,
	pub children: Vec<Box<dyn Widget>>,
}

impl FlexBox {
	pub fn row() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Row,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			children: vec![],
		}
	}
	pub fn col() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Col,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			children: vec![],
		}
	}
	pub fn wrap(mut self, wrap: FlexWrap) -> Self {
		self.wrap = wrap;
		self
	}
	pub fn align_items(mut self, align: AlignItems) -> Self {
		self.align_items = align;
		self
	}
	pub fn child(mut self, child: impl 'static + Widget) -> Self {
		self.children.push(Box::new(child));
		self
	}

	// ── private helpers ───────────────────────────────────────────────────────

	/// Greedy line-forming pass.
	/// Returns Vec of lines; each line is Vec<(child_index, natural_size)>.
	fn form_lines(&self, available: Size) -> Vec<Vec<(usize, Size)>> {
		let container_main = match self.direction {
			Direction::Row => available.w,
			Direction::Col => available.h,
		};

		let mut lines: Vec<Vec<(usize, Size)>> = vec![];
		let mut current: Vec<(usize, Size)> = vec![];
		let mut main_used = 0u16;

		for (i, child) in self.children.iter().enumerate() {
			let size = child.measure(available);
			let child_main = main_size(size, self.direction);

			// Would adding this child overflow the current line?
			let overflows = self.wrap == FlexWrap::Wrap
				&& !current.is_empty()
				&& main_used.saturating_add(child_main) > container_main;

			if overflows {
				lines.push(std::mem::take(&mut current));
				main_used = 0;
			}

			main_used += child_main;
			current.push((i, size));
		}
		if !current.is_empty() {
			lines.push(current);
		}
		lines
	}

	/// Distribute free main-axis space among children with flex_grow > 0.
	fn apply_flex_grow(
		&self,
		line: &[(usize, Size)],
		container_main: u16,
	) -> Vec<Size> {
		let natural_total: u16 = line
			.iter()
			.map(|(_, s)| main_size(*s, self.direction))
			.sum();

		let free = container_main.saturating_sub(natural_total);
		let total_grow: u16 = line
			.iter()
			.map(|(i, _)| {
				self.children[*i]
					.layout_style()
					.flex_grow
					.unwrap_or_default()
			})
			.sum();

		line.iter()
			.map(|(idx, nat)| {
				let grow = self.children[*idx]
					.layout_style()
					.flex_grow
					.unwrap_or_default();
				let bonus = if total_grow > 0 {
					(free as u32 * grow as u32 / total_grow as u32) as u16
				} else {
					0
				};

				match self.direction {
					Direction::Row => Size {
						w: nat.w + bonus,
						h: nat.h,
					},
					Direction::Col => Size {
						w: nat.w,
						h: nat.h + bonus,
					},
				}
			})
			.collect()
	}

	/// Cross-axis size of a line = max of children's cross sizes.
	fn line_cross_size(&self, sizes: &[Size]) -> u16 {
		sizes
			.iter()
			.map(|s| cross_size(*s, self.direction))
			.max()
			.unwrap_or(0)
	}

	/// Offset of a child within a line along the cross axis.
	fn cross_offset(&self, child_cross: u16, line_cross: u16) -> u16 {
		match self.align_items {
			AlignItems::Start | AlignItems::Stretch => 0,
			AlignItems::Center => line_cross.saturating_sub(child_cross) / 2,
			AlignItems::End => line_cross.saturating_sub(child_cross),
		}
	}
}

impl Widget for FlexBox {
	fn layout_style(&self) -> &LayoutStyle { &self.layout_style }
	fn measure(&self, available: Size) -> Size {
		let lines = self.form_lines(available);
		match self.direction {
			Direction::Row => {
				// Lines stack vertically → total_h = sum of line heights
				//                          total_w = max of line widths
				lines.iter().fold(Size::default(), |acc, line| {
					let lw: u16 = line.iter().map(|(_, s)| s.w).sum();
					let lh: u16 =
						line.iter().map(|(_, s)| s.h).max().unwrap_or(0);
					Size {
						w: acc.w.max(lw),
						h: acc.h.saturating_add(lh),
					}
				})
			}
			Direction::Col => {
				// Lines (columns) sit side by side → total_w = sum of line widths
				//                                    total_h = max of line heights
				lines.iter().fold(Size::default(), |acc, line| {
					let lh: u16 = line.iter().map(|(_, s)| s.h).sum();
					let lw: u16 =
						line.iter().map(|(_, s)| s.w).max().unwrap_or(0);
					Size {
						w: acc.w.saturating_add(lw),
						h: acc.h.max(lh),
					}
				})
			}
		}
	}

	fn layout(&self, rect: Rect, out: &mut Vec<Cell>) {
		let lines = self.form_lines(rect.size());

		match self.direction {
			// ── Row layout ────────────────────────────────────────────────────
			// Each line is a horizontal strip. Lines stack top-to-bottom.
			Direction::Row => {
				let mut cursor_y = rect.y;

				for line in &lines {
					if cursor_y >= rect.y + rect.h {
						break;
					}

					let final_sizes = self.apply_flex_grow(line, rect.w);
					let line_h = self
						.line_cross_size(&final_sizes)
						.min(rect.y + rect.h - cursor_y);

					let mut cursor_x = rect.x;
					for ((idx, _), fsize) in line.iter().zip(final_sizes.iter())
					{
						// Stretch overrides the child's natural cross size
						let child_h = match self.align_items {
							AlignItems::Stretch => line_h,
							_ => fsize.h.min(line_h),
						};
						let child_y =
							cursor_y + self.cross_offset(child_h, line_h);

						self.children[*idx].layout(
							Rect {
								x: cursor_x,
								y: child_y,
								w: fsize.w,
								h: child_h,
							},
							out,
						);
						cursor_x += fsize.w;
					}
					cursor_y += line_h;
				}
			}

			// ── Col layout ────────────────────────────────────────────────────
			// Each "line" is a vertical column. Columns sit left-to-right.
			Direction::Col => {
				let mut cursor_x = rect.x;

				for line in &lines {
					if cursor_x >= rect.x + rect.w {
						break;
					}

					let final_sizes = self.apply_flex_grow(line, rect.h);
					let line_w = self
						.line_cross_size(&final_sizes)
						.min(rect.x + rect.w - cursor_x);

					let mut cursor_y = rect.y;
					for ((idx, _), fsize) in line.iter().zip(final_sizes.iter())
					{
						let child_w = match self.align_items {
							AlignItems::Stretch => line_w,
							_ => fsize.w.min(line_w),
						};
						let child_x =
							cursor_x + self.cross_offset(child_w, line_w);

						self.children[*idx].layout(
							Rect {
								x: child_x,
								y: cursor_y,
								w: child_w,
								h: fsize.h,
							},
							out,
						);
						cursor_y += fsize.h;
					}
					cursor_x += line_w;
				}
			}
		}
	}
}

// ── Direction helpers ─────────────────────────────────────────────────────────

fn main_size(s: Size, dir: Direction) -> u16 {
	match dir {
		Direction::Row => s.w,
		Direction::Col => s.h,
	}
}

fn cross_size(s: Size, dir: Direction) -> u16 {
	match dir {
		Direction::Row => s.h,
		Direction::Col => s.w,
	}
}
