use crate::prelude::*;
use crate::style::AlignContent;
use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::FlexWrap;
use crate::style::JustifyContent;
use crate::style::LayoutStyle;
use beet_core::prelude::*;
use bevy::ecs::component::Component;
use bevy::math::URect;
use bevy::math::UVec2;
use bevy::prelude::Result;

#[derive(Component)]
pub struct FlexBox {
	pub direction: Direction,
	pub layout_style: LayoutStyle,
	pub wrap: FlexWrap,
	pub align_items: AlignItems,
	pub align_content: AlignContent,
	pub justify_content: JustifyContent,
	pub row_gap: u32,
	pub column_gap: u32,
}

impl FlexBox {
	pub fn row() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Horizontal,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
		}
	}
	pub fn col() -> Self {
		Self {
			layout_style: LayoutStyle::default(),
			direction: Direction::Vertical,
			wrap: FlexWrap::NoWrap,
			align_items: AlignItems::default(),
			align_content: AlignContent::default(),
			justify_content: JustifyContent::default(),
			row_gap: 0,
			column_gap: 0,
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
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Resolves the effective direction based on viewport dimensions
fn resolve_direction(direction: Direction, viewport: URect) -> Direction {
	match direction {
		Direction::ViewportMin => {
			if viewport.width() <= viewport.height() {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		Direction::ViewportMax => {
			if viewport.width() >= viewport.height() {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		_ => direction,
	}
}

fn main_size(size: UVec2, direction: Direction, viewport: URect) -> u32 {
	let dir = resolve_direction(direction, viewport);
	match dir {
		Direction::Horizontal => size.x,
		Direction::Vertical => size.y,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

fn cross_size(size: UVec2, direction: Direction, viewport: URect) -> u32 {
	let dir = resolve_direction(direction, viewport);
	match dir {
		Direction::Horizontal => size.y,
		Direction::Vertical => size.x,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

// ── ECS-based flex layout ────────────────────────────────────────────────────

/// Measure pass: calculate the size needed for this flexbox and its children.
pub fn flex_measure(
	node: &StyledNodeView,
	available: UVec2,
	viewport: URect,
) -> Result<UVec2> {
	let Some(flexbox) = node.flexbox else {
		return Ok(UVec2::ZERO);
	};

	let lines = form_lines_ecs(node, flexbox, available, viewport)?;

	let direction = resolve_direction(flexbox.direction, viewport);
	let line_gap = match direction {
		Direction::Horizontal => flexbox.row_gap,
		Direction::Vertical => flexbox.column_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	match direction {
		Direction::Horizontal => {
			// lines stack vertically → total_h = sum of line heights, total_w = max of line widths
			let mut total_h = 0u32;
			let mut max_w = 0u32;
			for (i, line) in lines.iter().enumerate() {
				if i > 0 {
					total_h += line_gap;
				}
				let gap_total = if line.len() > 1 {
					flexbox.column_gap * (line.len() as u32 - 1)
				} else {
					0
				};
				let lw: u32 =
					line.iter().map(|(_, s)| s.x).sum::<u32>() + gap_total;
				let lh: u32 = line.iter().map(|(_, s)| s.y).max().unwrap_or(0);
				max_w = max_w.max(lw);
				total_h = total_h.saturating_add(lh);
			}
			UVec2::new(max_w, total_h).xok()
		}
		Direction::Vertical => {
			// lines (columns) sit side by side → total_w = sum of line widths, total_h = max of line heights
			let mut total_w = 0u32;
			let mut max_h = 0u32;
			for (i, line) in lines.iter().enumerate() {
				if i > 0 {
					total_w += line_gap;
				}
				let gap_total = if line.len() > 1 {
					flexbox.row_gap * (line.len() as u32 - 1)
				} else {
					0
				};
				let lh: u32 =
					line.iter().map(|(_, s)| s.y).sum::<u32>() + gap_total;
				let lw: u32 = line.iter().map(|(_, s)| s.x).max().unwrap_or(0);
				total_w = total_w.saturating_add(lw);
				max_h = max_h.max(lh);
			}
			UVec2::new(total_w, max_h).xok()
		}
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

/// Layout pass: position and render flexbox children.
pub fn flex_layout(cx: &mut TuiRenderContext) -> Result<()> {
	let Some(flexbox) = cx.node.flexbox else {
		return Ok(());
	};

	let available = UVec2::new(cx.rect.width(), cx.rect.height());
	let lines = form_lines_ecs(&cx.node, flexbox, available, cx.viewport)?;

	// collect line cross sizes
	let line_cross_sizes: Vec<u32> = lines
		.iter()
		.map(|line| {
			let sizes: Vec<UVec2> = line.iter().map(|(_, s)| *s).collect();
			line_cross_size_for(&sizes, flexbox.direction, cx.viewport)
		})
		.collect();

	let direction = resolve_direction(flexbox.direction, cx.viewport);
	let container_cross = match direction {
		Direction::Horizontal => cx.rect.height(),
		Direction::Vertical => cx.rect.width(),
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let line_positions = apply_align_content(
		flexbox,
		&line_cross_sizes,
		container_cross,
		cx.viewport,
	);

	match direction {
		// ── Row layout ──────────────────────────────────────────────────────
		// Each line is a horizontal strip. Lines stack top-to-bottom.
		Direction::Horizontal => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_y = cx.rect.min.y + line_positions[line_idx];
				let line_h = if flexbox.align_content == AlignContent::Stretch {
					let bonus = (container_cross
						- line_cross_sizes.iter().sum::<u32>()
						- if line_cross_sizes.len() > 1 {
							flexbox.row_gap
								* (line_cross_sizes.len() as u32 - 1)
						} else {
							0
						}) / line_cross_sizes.len() as u32;
					line_cross_sizes[line_idx] + bonus
				} else {
					line_cross_sizes[line_idx]
				};

				if line_y >= cx.rect.max.y {
					break;
				}

				let final_sizes = apply_flex_grow(
					cx.node,
					flexbox,
					line,
					cx.rect.width(),
					cx.viewport,
				);
				let main_positions = apply_justify(
					flexbox,
					line,
					&final_sizes,
					cx.rect.width(),
					cx.viewport,
				);

				for (item_idx, ((child_node, _), fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let align = resolve_align(&child_node, flexbox.align_items);
					// stretch overrides the child's natural cross size
					let child_h = match align {
						AlignItems::Stretch => line_h,
						_ => fsize.y.min(line_h),
					};
					let child_y = line_y
						+ cross_offset(&child_node, flexbox, child_h, line_h);
					let child_x = cx.rect.min.x + main_positions[item_idx];

					let child_rect = URect::new(
						child_x,
						child_y,
						child_x + fsize.x,
						child_y + child_h,
					);

					// render child
					let mut child_cx = TuiRenderContext {
						query: cx.query,
						node: &child_node,
						viewport: cx.viewport,
						rect: child_rect,
						buffer: cx.buffer,
					};
					render_node(&mut child_cx)?;
				}
			}
		}

		// ── Col layout ──────────────────────────────────────────────────────
		// Each "line" is a vertical column. Columns sit left-to-right.
		Direction::Vertical => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_x = cx.rect.min.x + line_positions[line_idx];
				let line_w = if flexbox.align_content == AlignContent::Stretch {
					let bonus = (container_cross
						- line_cross_sizes.iter().sum::<u32>()
						- if line_cross_sizes.len() > 1 {
							flexbox.column_gap
								* (line_cross_sizes.len() as u32 - 1)
						} else {
							0
						}) / line_cross_sizes.len() as u32;
					line_cross_sizes[line_idx] + bonus
				} else {
					line_cross_sizes[line_idx]
				};

				if line_x >= cx.rect.max.x {
					break;
				}

				let final_sizes = apply_flex_grow(
					cx.node,
					flexbox,
					line,
					cx.rect.height(),
					cx.viewport,
				);
				let main_positions = apply_justify(
					flexbox,
					line,
					&final_sizes,
					cx.rect.height(),
					cx.viewport,
				);

				for (item_idx, ((child_node, _), fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let align = resolve_align(&child_node, flexbox.align_items);
					let child_w = match align {
						AlignItems::Stretch => line_w,
						_ => fsize.x.min(line_w),
					};
					let child_x = line_x
						+ cross_offset(&child_node, flexbox, child_w, line_w);
					let child_y = cx.rect.min.y + main_positions[item_idx];

					let child_rect = URect::new(
						child_x,
						child_y,
						child_x + child_w,
						child_y + fsize.y,
					);

					// render child
					let mut child_cx = TuiRenderContext {
						query: cx.query,
						node: &child_node,
						viewport: cx.viewport,
						rect: child_rect,
						buffer: cx.buffer,
					};
					render_node(&mut child_cx)?;
				}
			}
		}
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
	Ok(())
}

fn form_lines_ecs<'a>(
	node: &StyledNodeView<'a>,
	flexbox: &FlexBox,
	available: UVec2,
	viewport: URect,
) -> Result<Vec<Vec<(StyledNodeView<'a>, UVec2)>>> {
	let direction = resolve_direction(flexbox.direction, viewport);
	let container_main = match direction {
		Direction::Horizontal => available.x,
		Direction::Vertical => available.y,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let mut lines: Vec<Vec<(StyledNodeView, UVec2)>> = vec![];
	let mut current: Vec<(StyledNodeView, UVec2)> = vec![];
	let mut main_used = 0u32;

	let main_gap = match direction {
		Direction::Horizontal => flexbox.column_gap,
		Direction::Vertical => flexbox.row_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	// sort children by flex_order
	let mut sorted = node.children.clone();
	sorted.sort_by_key(|child| child.layout.map(|l| l.flex_order).unwrap_or(0));

	for child in sorted {
		let size = measure_node(&child, available, viewport)?;
		let child_main = main_size(size, flexbox.direction, viewport);

		// account for gap between items
		let gap_cost = if current.is_empty() { 0 } else { main_gap };

		// would adding this child overflow the current line?
		let overflows = flexbox.wrap == FlexWrap::Wrap
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
		current.push((child, size));
	}
	if !current.is_empty() {
		lines.push(current);
	}
	lines.xok()
}

/// Measure a single node (recursively measuring flex children if needed).
fn measure_node(
	node: &StyledNodeView,
	available: UVec2,
	viewport: URect,
) -> Result<UVec2> {
	// if node has flexbox, use flex_measure
	if node.flexbox.is_some() {
		return flex_measure(node, available, viewport);
	}
	// if node has text, use text_measure
	if node.value.is_some() {
		return text_measure(node, available);
	}
	// otherwise return zero size
	UVec2::ZERO.xok()
}

fn line_cross_size_for(
	sizes: &[UVec2],
	direction: Direction,
	viewport: URect,
) -> u32 {
	sizes
		.iter()
		.map(|s| cross_size(*s, direction, viewport))
		.max()
		.unwrap_or(0)
}

fn resolve_align(
	node: &StyledNodeView,
	default_align: AlignItems,
) -> AlignItems {
	let align_self = node
		.layout
		.map(|l| l.align_self.clone())
		.unwrap_or(AlignSelf::Auto);
	match align_self {
		AlignSelf::Auto => default_align,
		AlignSelf::Start => AlignItems::Start,
		AlignSelf::Center => AlignItems::Center,
		AlignSelf::End => AlignItems::End,
		AlignSelf::Stretch => AlignItems::Stretch,
		AlignSelf::Baseline => todo!(),
	}
}

fn cross_offset(
	node: &StyledNodeView,
	flexbox: &FlexBox,
	child_cross: u32,
	line_cross: u32,
) -> u32 {
	let align = resolve_align(node, flexbox.align_items);
	match align {
		AlignItems::Start | AlignItems::Stretch => 0,
		AlignItems::Center => line_cross.saturating_sub(child_cross) / 2,
		AlignItems::End => line_cross.saturating_sub(child_cross),
		AlignItems::Baseline => todo!(),
	}
}

fn apply_flex_grow(
	node: &StyledNodeView,
	flexbox: &FlexBox,
	line: &[(StyledNodeView, UVec2)],
	container_main: u32,
	viewport: URect,
) -> Vec<UVec2> {
	let direction = resolve_direction(flexbox.direction, viewport);
	let main_gap = match direction {
		Direction::Horizontal => flexbox.column_gap,
		Direction::Vertical => flexbox.row_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let gap_total = if line.len() > 1 {
		main_gap * (line.len() as u32 - 1)
	} else {
		0
	};

	let natural_total: u32 = line
		.iter()
		.map(|(_, s)| main_size(*s, flexbox.direction, viewport))
		.sum();

	let free = container_main.saturating_sub(natural_total + gap_total);

	// collect flex_grow values from children
	let grow_values: Vec<u32> = line
		.iter()
		.map(|(line_node, _)| {
			let child = node
				.children
				.iter()
				.find(|c| c.entity == line_node.entity)
				.unwrap();
			child.layout.map(|l| l.flex_grow).unwrap_or(0)
		})
		.collect();

	let total_grow: u32 = grow_values.iter().sum();

	line.iter()
		.zip(grow_values.iter())
		.map(|((_, nat), &grow)| {
			let bonus = if total_grow > 0 {
				(free as u64 * grow as u64 / total_grow as u64) as u32
			} else {
				0
			};

			match direction {
				Direction::Horizontal => UVec2::new(nat.x + bonus, nat.y),
				Direction::Vertical => UVec2::new(nat.x, nat.y + bonus),
				_ => unreachable!(
					"resolve_direction should eliminate viewport variants"
				),
			}
		})
		.collect()
}

fn apply_justify(
	flexbox: &FlexBox,
	line: &[(StyledNodeView, UVec2)],
	final_sizes: &[UVec2],
	container_main: u32,
	viewport: URect,
) -> Vec<u32> {
	let direction = resolve_direction(flexbox.direction, viewport);
	let main_gap = match direction {
		Direction::Horizontal => flexbox.column_gap,
		Direction::Vertical => flexbox.row_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let gap_total = if line.len() > 1 {
		main_gap * (line.len() as u32 - 1)
	} else {
		0
	};

	let total_main: u32 = final_sizes
		.iter()
		.map(|s| main_size(*s, flexbox.direction, viewport))
		.sum();

	let free = container_main.saturating_sub(total_main + gap_total);

	let mut positions = Vec::new();
	match flexbox.justify_content {
		JustifyContent::Start => {
			let mut pos = 0;
			for size in final_sizes {
				positions.push(pos);
				pos += main_size(*size, flexbox.direction, viewport) + main_gap;
			}
		}
		JustifyContent::End => {
			let mut pos = free;
			for size in final_sizes {
				positions.push(pos);
				pos += main_size(*size, flexbox.direction, viewport) + main_gap;
			}
		}
		JustifyContent::Center => {
			let mut pos = free / 2;
			for size in final_sizes {
				positions.push(pos);
				pos += main_size(*size, flexbox.direction, viewport) + main_gap;
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
					pos +=
						main_size(*size, flexbox.direction, viewport) + spacing;
				}
			}
		}
		JustifyContent::SpaceAround => {
			let spacing = free / final_sizes.len() as u32;
			let mut pos = spacing / 2;
			for size in final_sizes {
				positions.push(pos);
				pos += main_size(*size, flexbox.direction, viewport) + spacing;
			}
		}
		JustifyContent::SpaceEvenly => {
			let spacing = free / (final_sizes.len() as u32 + 1);
			let mut pos = spacing;
			for size in final_sizes {
				positions.push(pos);
				pos += main_size(*size, flexbox.direction, viewport) + spacing;
			}
		}
	}
	positions
}

fn apply_align_content(
	flexbox: &FlexBox,
	line_cross_sizes: &[u32],
	container_cross: u32,
	viewport: URect,
) -> Vec<u32> {
	let direction = resolve_direction(flexbox.direction, viewport);
	let line_gap = match direction {
		Direction::Horizontal => flexbox.row_gap,
		Direction::Vertical => flexbox.column_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let gap_total = if line_cross_sizes.len() > 1 {
		line_gap * (line_cross_sizes.len() as u32 - 1)
	} else {
		0
	};

	let total_cross: u32 = line_cross_sizes.iter().sum();
	let free = container_cross.saturating_sub(total_cross + gap_total);

	let mut positions = Vec::new();
	match flexbox.align_content {
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
