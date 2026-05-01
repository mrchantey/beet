use super::*;
use bevy::math::URect;
use bevy::math::UVec2;

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

/// How to distribute children along the main axis.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum JustifyContent {
	#[default]
	Start,
	Center,
	End,
	SpaceBetween,
	SpaceAround,
	SpaceEvenly,
}

/// How to distribute lines along the cross axis when wrapping.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AlignContent {
	#[default]
	Start,
	Center,
	End,
	SpaceBetween,
	SpaceAround,
	Stretch,
}

/// Individual item alignment override.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AlignSelf {
	#[default]
	Auto, // inherit from container's align_items
	Start,
	Center,
	End,
	Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum FlexWrap {
	#[default]
	NoWrap,
	Wrap,
}

/// Spacing around an element.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spacing {
	pub top: u32,
	pub right: u32,
	pub bottom: u32,
	pub left: u32,
}

impl Spacing {
	pub fn all(value: u32) -> Self {
		Self {
			top: value,
			right: value,
			bottom: value,
			left: value,
		}
	}
	pub fn horizontal(&self) -> u32 { self.left + self.right }
	pub fn vertical(&self) -> u32 { self.top + self.bottom }
}

// ── LayoutStyle ───────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct LayoutStyle {
	pub flex_order: i32,
	pub flex_grow: u32,
	pub align_self: AlignSelf,
	pub padding: Spacing,
	pub margin: Spacing,
}

// ── FlexBox ───────────────────────────────────────────────────────────────────

pub struct FlexBox {
	pub direction: Direction,
	pub layout_style: LayoutStyle,
	pub wrap: FlexWrap,
	pub align_items: AlignItems,
	pub align_content: AlignContent,
	pub justify_content: JustifyContent,
	pub row_gap: u32,
	pub column_gap: u32,
	pub children: Vec<Box<dyn 'static + Send + Sync + Widget>>,
}

impl FlexBox {
	pub fn row() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Row,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
			children: vec![],
		}
	}
	pub fn col() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Col,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
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
	pub fn align_content(mut self, align: AlignContent) -> Self {
		self.align_content = align;
		self
	}
	pub fn justify_content(mut self, justify: JustifyContent) -> Self {
		self.justify_content = justify;
		self
	}
	pub fn row_gap(mut self, gap: u32) -> Self {
		self.row_gap = gap;
		self
	}
	pub fn column_gap(mut self, gap: u32) -> Self {
		self.column_gap = gap;
		self
	}
	pub fn gap(mut self, gap: u32) -> Self {
		self.row_gap = gap;
		self.column_gap = gap;
		self
	}
	pub fn child(mut self, child: impl 'static + Send + Sync + Widget) -> Self {
		self.children.push(Box::new(child));
		self
	}

	// ── private helpers ───────────────────────────────────────────────────────

	fn main_gap(&self) -> u32 {
		match self.direction {
			Direction::Row => self.column_gap,
			Direction::Col => self.row_gap,
		}
	}

	/// Greedy line-forming pass.
	/// Returns Vec of lines; each line is Vec<(child_index, natural_size)>.
	fn form_lines(&self, available: UVec2) -> Vec<Vec<(usize, UVec2)>> {
		let container_main = match self.direction {
			Direction::Row => available.x,
			Direction::Col => available.y,
		};

		let mut lines: Vec<Vec<(usize, UVec2)>> = vec![];
		let mut current: Vec<(usize, UVec2)> = vec![];
		let mut main_used = 0u32;

		// sort children by flex_order
		let mut indices: Vec<usize> = (0..self.children.len()).collect();
		indices.sort_by_key(|&i| self.children[i].layout_style().flex_order);

		for &i in &indices {
			let child = &self.children[i];
			let size = child.measure(available);
			let child_main = main_size(size, self.direction);

			// account for gap between items
			let gap_cost = if current.is_empty() {
				0
			} else {
				self.main_gap()
			};

			// Would adding this child overflow the current line?
			let overflows = self.wrap == FlexWrap::Wrap
				&& !current.is_empty()
				&& main_used
					.saturating_add(gap_cost)
					.saturating_add(child_main)
					> container_main;

			if overflows {
				lines.push(std::mem::take(&mut current));
				main_used = 0;
			} else if !current.is_empty() {
				main_used += gap_cost;
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
		line: &[(usize, UVec2)],
		container_main: u32,
	) -> Vec<UVec2> {
		let gap_total = if line.len() > 1 {
			self.main_gap() * (line.len() as u32 - 1)
		} else {
			0
		};

		let natural_total: u32 = line
			.iter()
			.map(|(_, s)| main_size(*s, self.direction))
			.sum();

		let free = container_main.saturating_sub(natural_total + gap_total);
		let total_grow: u32 = line
			.iter()
			.map(|(i, _)| self.children[*i].layout_style().flex_grow)
			.sum();

		line.iter()
			.map(|(idx, nat)| {
				let grow = self.children[*idx].layout_style().flex_grow;
				let bonus = if total_grow > 0 {
					(free as u64 * grow as u64 / total_grow as u64) as u32
				} else {
					0
				};

				match self.direction {
					Direction::Row => UVec2::new(nat.x + bonus, nat.y),
					Direction::Col => UVec2::new(nat.x, nat.y + bonus),
				}
			})
			.collect()
	}

	/// Cross-axis size of a line = max of children's cross sizes.
	fn line_cross_size(&self, sizes: &[UVec2]) -> u32 {
		sizes
			.iter()
			.map(|s| cross_size(*s, self.direction))
			.max()
			.unwrap_or(0)
	}

	/// Resolve align_self for a child, defaulting to container's align_items.
	fn resolve_align(&self, child_idx: usize) -> AlignItems {
		match self.children[child_idx].layout_style().align_self {
			AlignSelf::Auto => self.align_items,
			AlignSelf::Start => AlignItems::Start,
			AlignSelf::Center => AlignItems::Center,
			AlignSelf::End => AlignItems::End,
			AlignSelf::Stretch => AlignItems::Stretch,
		}
	}

	/// Offset of a child within a line along the cross axis.
	fn cross_offset(
		&self,
		child_idx: usize,
		child_cross: u32,
		line_cross: u32,
	) -> u32 {
		let align = self.resolve_align(child_idx);
		match align {
			AlignItems::Start | AlignItems::Stretch => 0,
			AlignItems::Center => line_cross.saturating_sub(child_cross) / 2,
			AlignItems::End => line_cross.saturating_sub(child_cross),
		}
	}

	/// Distribute children along the main axis using justify_content.
	fn apply_justify(
		&self,
		line: &[(usize, UVec2)],
		final_sizes: &[UVec2],
		container_main: u32,
	) -> Vec<u32> {
		let gap_total = if line.len() > 1 {
			self.main_gap() * (line.len() as u32 - 1)
		} else {
			0
		};

		let total_main: u32 = final_sizes
			.iter()
			.map(|s| main_size(*s, self.direction))
			.sum();

		let free = container_main.saturating_sub(total_main + gap_total);

		let mut positions = Vec::new();
		match self.justify_content {
			JustifyContent::Start => {
				let mut pos = 0;
				for size in final_sizes {
					positions.push(pos);
					pos += main_size(*size, self.direction) + self.main_gap();
				}
			}
			JustifyContent::End => {
				let mut pos = free;
				for size in final_sizes {
					positions.push(pos);
					pos += main_size(*size, self.direction) + self.main_gap();
				}
			}
			JustifyContent::Center => {
				let mut pos = free / 2;
				for size in final_sizes {
					positions.push(pos);
					pos += main_size(*size, self.direction) + self.main_gap();
				}
			}
			JustifyContent::SpaceBetween => {
				if final_sizes.len() <= 1 {
					positions.push(0);
				} else {
					let spacing = free / (final_sizes.len() as u32 - 1);
					let mut pos = 0;
					for size in final_sizes {
						positions.push(pos);
						pos += main_size(*size, self.direction) + spacing;
					}
				}
			}
			JustifyContent::SpaceAround => {
				let spacing = free / final_sizes.len() as u32;
				let mut pos = spacing / 2;
				for size in final_sizes {
					positions.push(pos);
					pos += main_size(*size, self.direction) + spacing;
				}
			}
			JustifyContent::SpaceEvenly => {
				let spacing = free / (final_sizes.len() as u32 + 1);
				let mut pos = spacing;
				for size in final_sizes {
					positions.push(pos);
					pos += main_size(*size, self.direction) + spacing;
				}
			}
		}
		positions
	}

	/// Distribute lines along the cross axis using align_content.
	fn apply_align_content(
		&self,
		line_cross_sizes: &[u32],
		container_cross: u32,
	) -> Vec<u32> {
		let line_gap = match self.direction {
			Direction::Row => self.row_gap,
			Direction::Col => self.column_gap,
		};

		let gap_total = if line_cross_sizes.len() > 1 {
			line_gap * (line_cross_sizes.len() as u32 - 1)
		} else {
			0
		};

		let total_cross: u32 = line_cross_sizes.iter().sum();
		let free = container_cross.saturating_sub(total_cross + gap_total);

		let mut positions = Vec::new();
		match self.align_content {
			AlignContent::Start => {
				let mut pos = 0;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + line_gap;
				}
			}
			AlignContent::End => {
				let mut pos = free;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + line_gap;
				}
			}
			AlignContent::Center => {
				let mut pos = free / 2;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + line_gap;
				}
			}
			AlignContent::SpaceBetween => {
				if line_cross_sizes.len() <= 1 {
					positions.push(0);
				} else {
					let spacing = free / (line_cross_sizes.len() as u32 - 1);
					let mut pos = 0;
					for &size in line_cross_sizes {
						positions.push(pos);
						pos += size + spacing;
					}
				}
			}
			AlignContent::SpaceAround => {
				let spacing = free / line_cross_sizes.len() as u32;
				let mut pos = spacing / 2;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + spacing;
				}
			}
			AlignContent::Stretch => {
				let bonus = free / line_cross_sizes.len() as u32;
				let mut pos = 0;
				for &size in line_cross_sizes {
					positions.push(pos);
					pos += size + bonus + line_gap;
				}
			}
		}
		positions
	}
}

impl Widget for FlexBox {
	fn layout_style(&self) -> &LayoutStyle { &self.layout_style }

	fn measure(&self, available: UVec2) -> UVec2 {
		let lines = self.form_lines(available);

		let line_gap = match self.direction {
			Direction::Row => self.row_gap,
			Direction::Col => self.column_gap,
		};

		match self.direction {
			Direction::Row => {
				// Lines stack vertically → total_h = sum of line heights
				//                          total_w = max of line widths
				let mut total_h = 0u32;
				let mut max_w = 0u32;
				for (i, line) in lines.iter().enumerate() {
					if i > 0 {
						total_h += line_gap;
					}
					let gap_total = if line.len() > 1 {
						self.column_gap * (line.len() as u32 - 1)
					} else {
						0
					};
					let lw: u32 =
						line.iter().map(|(_, s)| s.x).sum::<u32>() + gap_total;
					let lh: u32 =
						line.iter().map(|(_, s)| s.y).max().unwrap_or(0);
					max_w = max_w.max(lw);
					total_h = total_h.saturating_add(lh);
				}
				UVec2::new(max_w, total_h)
			}
			Direction::Col => {
				// Lines (columns) sit side by side → total_w = sum of line widths
				//                                    total_h = max of line heights
				let mut total_w = 0u32;
				let mut max_h = 0u32;
				for (i, line) in lines.iter().enumerate() {
					if i > 0 {
						total_w += line_gap;
					}
					let gap_total = if line.len() > 1 {
						self.row_gap * (line.len() as u32 - 1)
					} else {
						0
					};
					let lh: u32 =
						line.iter().map(|(_, s)| s.y).sum::<u32>() + gap_total;
					let lw: u32 =
						line.iter().map(|(_, s)| s.x).max().unwrap_or(0);
					total_w = total_w.saturating_add(lw);
					max_h = max_h.max(lh);
				}
				UVec2::new(total_w, max_h)
			}
		}
	}

	fn layout(&self, buffer: &mut Buffer, rect: URect) {
		let available = UVec2::new(rect.width(), rect.height());
		let lines = self.form_lines(available);

		// collect line cross sizes
		let line_cross_sizes: Vec<u32> = lines
			.iter()
			.map(|line| {
				let sizes: Vec<UVec2> = line.iter().map(|(_, s)| *s).collect();
				self.line_cross_size(&sizes)
			})
			.collect();

		let container_cross = match self.direction {
			Direction::Row => rect.height(),
			Direction::Col => rect.width(),
		};

		let line_positions =
			self.apply_align_content(&line_cross_sizes, container_cross);

		match self.direction {
			// ── Row layout ────────────────────────────────────────────────────
			// Each line is a horizontal strip. Lines stack top-to-bottom.
			Direction::Row => {
				for (line_idx, line) in lines.iter().enumerate() {
					let line_y = rect.min.y + line_positions[line_idx];
					let line_h = if self.align_content == AlignContent::Stretch
					{
						let bonus = (container_cross
							- line_cross_sizes.iter().sum::<u32>()
							- if line_cross_sizes.len() > 1 {
								self.row_gap
									* (line_cross_sizes.len() as u32 - 1)
							} else {
								0
							}) / line_cross_sizes.len() as u32;
						line_cross_sizes[line_idx] + bonus
					} else {
						line_cross_sizes[line_idx]
					};

					if line_y >= rect.max.y {
						break;
					}

					let final_sizes = self.apply_flex_grow(line, rect.width());
					let main_positions =
						self.apply_justify(line, &final_sizes, rect.width());

					for (item_idx, ((idx, _), fsize)) in
						line.iter().zip(final_sizes.iter()).enumerate()
					{
						let align = self.resolve_align(*idx);
						// Stretch overrides the child's natural cross size
						let child_h = match align {
							AlignItems::Stretch => line_h,
							_ => fsize.y.min(line_h),
						};
						let child_y =
							line_y + self.cross_offset(*idx, child_h, line_h);
						let child_x = rect.min.x + main_positions[item_idx];

						let child_rect = URect::new(
							child_x,
							child_y,
							child_x + fsize.x,
							child_y + child_h,
						);

						self.children[*idx].layout(buffer, child_rect);
					}
				}
			}

			// ── Col layout ────────────────────────────────────────────────────
			// Each "line" is a vertical column. Columns sit left-to-right.
			Direction::Col => {
				for (line_idx, line) in lines.iter().enumerate() {
					let line_x = rect.min.x + line_positions[line_idx];
					let line_w = if self.align_content == AlignContent::Stretch
					{
						let bonus = (container_cross
							- line_cross_sizes.iter().sum::<u32>()
							- if line_cross_sizes.len() > 1 {
								self.column_gap
									* (line_cross_sizes.len() as u32 - 1)
							} else {
								0
							}) / line_cross_sizes.len() as u32;
						line_cross_sizes[line_idx] + bonus
					} else {
						line_cross_sizes[line_idx]
					};

					if line_x >= rect.max.x {
						break;
					}

					let final_sizes = self.apply_flex_grow(line, rect.height());
					let main_positions =
						self.apply_justify(line, &final_sizes, rect.height());

					for (item_idx, ((idx, _), fsize)) in
						line.iter().zip(final_sizes.iter()).enumerate()
					{
						let align = self.resolve_align(*idx);
						let child_w = match align {
							AlignItems::Stretch => line_w,
							_ => fsize.x.min(line_w),
						};
						let child_x =
							line_x + self.cross_offset(*idx, child_w, line_w);
						let child_y = rect.min.y + main_positions[item_idx];

						let child_rect = URect::new(
							child_x,
							child_y,
							child_x + child_w,
							child_y + fsize.y,
						);

						self.children[*idx].layout(buffer, child_rect);
					}
				}
			}
		}
	}
}

// ── Direction helpers ─────────────────────────────────────────────────────────

fn main_size(s: UVec2, dir: Direction) -> u32 {
	match dir {
		Direction::Row => s.x,
		Direction::Col => s.y,
	}
}

fn cross_size(s: UVec2, dir: Direction) -> u32 {
	match dir {
		Direction::Row => s.y,
		Direction::Col => s.x,
	}
}
