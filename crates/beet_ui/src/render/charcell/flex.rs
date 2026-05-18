use super::BoxModel;
use crate::style::AlignContent;
use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::FlexBox;
use crate::style::FlexWrap;
use crate::style::JustifyContent;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

use super::query::CharcellNodeData;
use super::query::CharcellQuery;

// ── Helper functions ──────────────────────────────────────────────────────────

/// Resolve the effective direction based on viewport dimensions.
fn resolve_direction(direction: Direction, viewport: UVec2) -> Direction {
	match direction {
		Direction::ViewportMin => {
			if viewport.x <= viewport.y {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		Direction::ViewportMax => {
			if viewport.x >= viewport.y {
				Direction::Horizontal
			} else {
				Direction::Vertical
			}
		}
		_ => direction,
	}
}

fn main_size(size: UVec2, direction: Direction, viewport: UVec2) -> u32 {
	let dir = resolve_direction(direction, viewport);
	match dir {
		Direction::Horizontal => size.x,
		Direction::Vertical => size.y,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

fn cross_size(size: UVec2, direction: Direction, viewport: UVec2) -> u32 {
	let dir = resolve_direction(direction, viewport);
	match dir {
		Direction::Horizontal => size.y,
		Direction::Vertical => size.x,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

// ── Measure pass ──────────────────────────────────────────────────────────────

/// Measure pass: compute the intrinsic size of this flexbox and its children.
///
/// `sizes` contains pre-measured intrinsic sizes for all children (bottom-up).
/// `styles` is used to look up flex-order for line formation.
pub fn measure_flex(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	sizes: &HashMap<Entity, UVec2>,
	available: UVec2,
	viewport: UVec2,
) -> Result<UVec2> {
	let flexbox = node.flexbox();
	// Build (entity, size) pairs from child_nodes, using fresh sizes from HashMap
	let mut child_sizes = node
		.child_nodes(query)
		.map(|child| {
			let size = sizes
				.get(&child.entity)
				.copied()
				.expect("unreachable, postorder populated sizes hashmap");
			(child.entity, size)
		})
		.collect::<Vec<_>>();
	// Sort by flex_order
	child_sizes.sort_by_key(|(e, _)| {
		query
			.node(*e)
			.map(|n| n.layout_style().flex_order)
			.unwrap_or(0)
	});
	let lines = form_lines(&child_sizes, flexbox, available, viewport);

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
			// Lines stack vertically: total_h = sum of line heights, total_w = max line width
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
			// Lines (columns) sit side-by-side: total_w = sum of line widths, total_h = max line height
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

// ── Layout pass ───────────────────────────────────────────────────────────────

/// Layout pass: assign a [`LayoutRect`] to each flex child.
///
/// Reads pre-computed [`IntrinsicSize`] from the query and writes
/// each child's rect into `layout_rects`.
pub fn flex_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	let flexbox = node.flexbox();

	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let available = UVec2::new(content_rect.width(), content_rect.height());

	// Get child sizes directly from child_nodes (already computed by measure phase)
	let mut child_sizes: Vec<(Entity, UVec2)> = node
		.child_nodes(query)
		.map(|child| (child.entity, child.intrinsic_size()))
		.collect();
	// Sort by flex_order
	child_sizes.sort_by_key(|(e, _)| {
		query
			.node(*e)
			.map(|n| n.layout_style().flex_order)
			.unwrap_or(0)
	});
	let lines = form_lines(&child_sizes, flexbox, available, viewport);

	// Collect cross sizes per line
	let line_cross_sizes: Vec<u32> = lines
		.iter()
		.map(|line| {
			let sizes: Vec<UVec2> = line.iter().map(|(_, s)| *s).collect();
			line_cross_size_for(&sizes, flexbox.direction, viewport)
		})
		.collect();

	let direction = resolve_direction(flexbox.direction, viewport);
	let container_cross = match direction {
		Direction::Horizontal => content_rect.height(),
		Direction::Vertical => content_rect.width(),
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let line_positions = apply_align_content(
		flexbox,
		&line_cross_sizes,
		container_cross,
		viewport,
	);

	match direction {
		// ── Row layout ──────────────────────────────────────────────────────
		Direction::Horizontal => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_y = content_rect.min.y + line_positions[line_idx];
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

				if line_y >= content_rect.max.y {
					break;
				}

				let final_sizes = apply_flex_grow(
					flexbox,
					line,
					query,
					content_rect.width(),
					viewport,
				);
				let main_positions = apply_justify(
					flexbox,
					line,
					&final_sizes,
					content_rect.width(),
					viewport,
				);

				for (item_idx, ((entity, _), fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let entity = *entity;
					let align =
						resolve_align(entity, query, flexbox.align_items);
					let child_h = match align {
						AlignItems::Stretch => line_h,
						_ => fsize.y.min(line_h),
					};
					let child_y = line_y
						+ cross_offset(entity, query, flexbox, child_h, line_h);
					let child_x = content_rect.min.x + main_positions[item_idx];
					let child_rect = URect::new(
						child_x,
						child_y,
						child_x + fsize.x,
						child_y + child_h,
					);
					layout_rects.insert(entity, child_rect);
				}
			}
		}

		// ── Col layout ──────────────────────────────────────────────────────
		Direction::Vertical => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_x = content_rect.min.x + line_positions[line_idx];
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

				if line_x >= content_rect.max.x {
					break;
				}

				let final_sizes = apply_flex_grow(
					flexbox,
					line,
					query,
					content_rect.height(),
					viewport,
				);
				let main_positions = apply_justify(
					flexbox,
					line,
					&final_sizes,
					content_rect.height(),
					viewport,
				);

				for (item_idx, ((entity, _), fsize)) in
					line.iter().zip(final_sizes.iter()).enumerate()
				{
					let entity = *entity;
					let align =
						resolve_align(entity, query, flexbox.align_items);
					let child_w = match align {
						AlignItems::Stretch => line_w,
						_ => fsize.x.min(line_w),
					};
					let child_x = line_x
						+ cross_offset(entity, query, flexbox, child_w, line_w);
					let child_y = content_rect.min.y + main_positions[item_idx];
					let child_rect = URect::new(
						child_x,
						child_y,
						child_x + child_w,
						child_y + fsize.y,
					);
					layout_rects.insert(entity, child_rect);
				}
			}
		}
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
	Ok(())
}

// ── Line formation ────────────────────────────────────────────────────────────

/// Form flex lines from pre-sorted (entity, size) pairs.
fn form_lines(
	child_sizes: &[(Entity, UVec2)],
	flexbox: &FlexBox,
	available: UVec2,
	viewport: UVec2,
) -> Vec<Vec<(Entity, UVec2)>> {
	let direction = resolve_direction(flexbox.direction, viewport);
	let container_main = match direction {
		Direction::Horizontal => available.x,
		Direction::Vertical => available.y,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let main_gap = match direction {
		Direction::Horizontal => flexbox.column_gap,
		Direction::Vertical => flexbox.row_gap,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let mut lines: Vec<Vec<(Entity, UVec2)>> = vec![];
	let mut current: Vec<(Entity, UVec2)> = vec![];
	let mut main_used = 0u32;

	for &(entity, size) in child_sizes {
		let child_main = main_size(size, flexbox.direction, viewport);

		let gap_cost = if current.is_empty() { 0 } else { main_gap };

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
		current.push((entity, size));
	}
	if !current.is_empty() {
		lines.push(current);
	}
	lines
}

fn line_cross_size_for(
	sizes: &[UVec2],
	direction: Direction,
	viewport: UVec2,
) -> u32 {
	sizes
		.iter()
		.map(|s| cross_size(*s, direction, viewport))
		.max()
		.unwrap_or(0)
}

fn resolve_align(
	entity: Entity,
	query: &CharcellQuery,
	default_align: AlignItems,
) -> AlignItems {
	let align_self = query
		.node(entity)
		.map(|n| n.layout_style().align_self.clone())
		.unwrap_or(AlignSelf::Auto);
	match align_self {
		AlignSelf::Auto => default_align,
		AlignSelf::Start => AlignItems::Start,
		AlignSelf::Center => AlignItems::Center,
		AlignSelf::End => AlignItems::End,
		AlignSelf::Stretch => AlignItems::Stretch,
		AlignSelf::Baseline => unimplemented!(),
	}
}

fn cross_offset(
	entity: Entity,
	query: &CharcellQuery,
	flexbox: &FlexBox,
	child_cross: u32,
	line_cross: u32,
) -> u32 {
	let align = resolve_align(entity, query, flexbox.align_items);
	match align {
		AlignItems::Start | AlignItems::Stretch => 0,
		AlignItems::Center => line_cross.saturating_sub(child_cross) / 2,
		AlignItems::End => line_cross.saturating_sub(child_cross),
		AlignItems::Baseline => unimplemented!(),
	}
}

fn apply_flex_grow(
	flexbox: &FlexBox,
	line: &[(Entity, UVec2)],
	query: &CharcellQuery,
	container_main: u32,
	viewport: UVec2,
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

	// Collect flex_grow from each child's layout style
	let grow_values: Vec<u32> = line
		.iter()
		.map(|(entity, _)| {
			query
				.node(*entity)
				.map(|n| n.layout_style().flex_grow)
				.unwrap_or(0)
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
	line: &[(Entity, UVec2)],
	final_sizes: &[UVec2],
	container_main: u32,
	viewport: UVec2,
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
	viewport: UVec2,
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


#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;
	use crate::style::*;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_sized(UVec2::new(40, 20), bundle)
			.trim_lines()
			.replace(" ", "+")
	}

	fn bordered() -> BoxStyle {
		BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)))
	}

	#[beet_core::test]
	fn justify_start() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::Start)
				.column_gap(1),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn justify_end() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::End)
				.column_gap(1),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn justify_center() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::Center)
				.column_gap(1),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_gap() {
		render((LayoutStyle::flex_row().column_gap(3), children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
		]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn flex_grow_distributes_space() {
		let output =
			render((LayoutStyle::flex_row().column_gap(1), children![
				(rsx! {"A"}, bordered()),
				(
					rsx! {"B"},
					bordered(),
					LayoutStyle::default().with_flex_grow(1)
				),
				(rsx! {"C"}, bordered()),
			]));
		output.xpect_snapshot();
		// B should be wider than A and C
		let lines: Vec<&str> = output.lines().collect();
		let top = lines[0];
		// count dashes in each box segment to verify B is wider
		let segments: Vec<&str> = top.split('+').collect();
		segments.len().xpect_eq(3); // should have 3 boxes
		let b_width = segments[1].len();
		let a_width = segments[0].len();
		(b_width > a_width).xpect_true();
	}

	#[beet_core::test]
	fn align_items_center() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::Center)
				.column_gap(1),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn align_items_start() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::Start)
				.column_gap(1),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn align_items_end() {
		render((
			LayoutStyle::flex_row()
				.align_items(AlignItems::End)
				.column_gap(1),
			children![
				(
					LayoutStyle::flex_col(),
					children![
						(rsx! {"Tall"}, bordered()),
						(rsx! {"Item"}, bordered()),
					],
					bordered()
				),
				(rsx! {"Short"}, bordered()),
			],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn nested_flex() {
		render((LayoutStyle::flex_col().row_gap(1), children![
			(
				LayoutStyle::flex_row().column_gap(1),
				children![(rsx! {"A"}, bordered()), (rsx! {"B"}, bordered()),],
				bordered()
			),
			(
				LayoutStyle::flex_row().column_gap(1),
				children![(rsx! {"C"}, bordered()), (rsx! {"D"}, bordered()),],
				bordered()
			),
		]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn padding_with_content() {
		render((LayoutStyle::flex_row(), children![(
			rsx! {"X"},
			BoxStyle::default()
				.with_border(Spacing::all(Length::Rem(1.)))
				.with_padding(Spacing::all(Length::Rem(0.5)))
		)]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_with_multiple_items() {
		render((LayoutStyle::flex_col().row_gap(1), children![
			(rsx! {"First"}, bordered()),
			(rsx! {"Second"}, bordered()),
			(rsx! {"Third"}, bordered()),
		]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn nested_column_in_row() {
		render((LayoutStyle::flex_row(), children![(
			LayoutStyle::flex_col(),
			children![
				(rsx! {"A"}, bordered()),
				(rsx! {"B"}, bordered()),
				(rsx! {"C"}, bordered()),
			],
			bordered()
		)]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_without_gap() {
		render((LayoutStyle::flex_col(), children![
			(rsx! {"A"}, bordered()),
			(rsx! {"B"}, bordered()),
		]))
		.xpect_snapshot();
	}

	/// Nested rows and columns with background colors on leaf nodes.
	/// Verifies background ordering and multi-level flex layout.
	#[beet_core::test]
	fn nested_with_backgrounds() {
		let out = render((LayoutStyle::flex_col().row_gap(1), children![
			(
				LayoutStyle::flex_row().column_gap(1),
				children![
					(rsx! { "Left" }, bordered(), VisualStyle {
						background: Some(Color::srgb(0.2, 0.2, 0.5)),
						..default()
					},),
					(rsx! { "Right" }, bordered(), VisualStyle {
						background: Some(Color::srgb(0.5, 0.2, 0.2)),
						..default()
					},),
				],
				bordered()
			),
			(
				LayoutStyle::flex_row().column_gap(1),
				children![(rsx! { "Below" }, bordered(), VisualStyle {
					background: Some(Color::srgb(0.2, 0.5, 0.2)),
					..default()
				},),],
				bordered()
			),
		]));
		// both rows should appear - header row and second row
		out.as_str().xpect_contains("┌"); // at least one top-left corner
		// ensure trim_lines worked (output should not include excess blank rows)
		(out.lines().count() <= 12).xpect_true();
		// both rows rendered: check for both content strings
		out.as_str().xpect_contains("Left");
		out.xpect_contains("Below");
	}

	/// Wide CJK chars each occupy 2 terminal columns.
	/// The border width must account for the display width, not the char count.
	#[beet_core::test]
	fn wide_chars_layout() {
		let out = render((LayoutStyle::flex_row().column_gap(1), children![
			(rsx! { "中文" }, bordered()),
			(rsx! { "ＡＢＣ" }, bordered()),
		]));
		// "中文": 2 wide chars = 4 cols content → border top = ┌────┐
		out.as_str().xpect_contains("┌────┐");
		// "ＡＢＣ": 3 wide chars = 6 cols content → border top = ┌──────┐
		out.xpect_contains("┌──────┐");
	}
}
