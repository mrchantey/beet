use super::BoxModel;
use crate::style::AlignContent;
use crate::style::AlignItems;
use crate::style::AlignSelf;
use crate::style::Direction;
use crate::style::Display;
use crate::style::FlexBox;
use crate::style::FlexWrap;
use crate::style::JustifyContent;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::UVec2;

use super::establishes_inline_flow;
use super::explicit_box_size;
use super::marker_gutter;
use super::measure_grid;
use super::measure_inline_flow;
use super::measure_scaled;
use super::measure_text;
use super::node_bottom_margin;
use super::query::CharcellNodeData;
use super::query::CharcellQuery;
use super::resolve_table_height;
use super::scrollport_of;

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
			.unresolved_node(*e)
			.map(|n| n.layout_style().flex_order)
			.unwrap_or(0)
	});
	let lines = form_lines(&child_sizes, flexbox, available, viewport);

	let direction = resolve_direction(flexbox.direction, viewport);
	let line_gap = match direction {
		Direction::Horizontal => flexbox.row_gap_cells(viewport),
		Direction::Vertical => flexbox.column_gap_cells(viewport),
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
					flexbox.column_gap_cells(viewport) * (line.len() as u32 - 1)
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
					flexbox.row_gap_cells(viewport) * (line.len() as u32 - 1)
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

// ── Width-constrained height resolution ────────────────────────────────────────

/// Total height (rows, including the node's own box overhead) the node needs
/// when its border box is constrained to `width` columns.
///
/// The [measure pass](measure_node) sizes heights at the unconstrained viewport
/// width, so a node later laid out into a narrower column (eg a paragraph beside
/// a sidebar) wraps into more rows than were reserved, and paint clips the tail.
/// The layout pass calls this with each node's *assigned* width so the reserved
/// height matches what paint will flow.
pub(super) fn resolve_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	width: u32,
	viewport: UVec2,
) -> u32 {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_width = width.saturating_sub(overhead.x);
	let content_height = match node.layout_style().display {
		Display::None => 0,
		Display::Flex => {
			resolve_flex_height(node, query, content_width, viewport)
		}
		Display::Grid => {
			measure_grid(node, query, UVec2::new(content_width, 0), viewport).y
		}
		Display::Table => {
			resolve_table_height(node, query, content_width, viewport)
		}
		// a kitty raster's rows follow its aspect at the assigned width (an
		// explicit `height` overrides below, like every other display)
		_ if let Some(image) = node.kitty_image() => {
			image
				.cell_size_constrained(Some(content_width), None, content_width)
				.y
		}
		// text leaf (eg a paragraph's text node)
		_ if node.value().is_some() => measure_text(node, content_width).y,
		// container of inline content: flow descendants as wrapped text
		_ if establishes_inline_flow(node, query) => {
			measure_inline_flow(node, query, content_width).y
		}
		// block leaf whose content is generated (eg `<hr>`, `<img>` alt)
		_ if let Some(marker) =
			node.marker().filter(|_| !node.has_child_nodes(query)) =>
		{
			measure_scaled(node.visual_style(), marker, content_width).y
		}
		// block container: stack children, each flowed at the constrained width
		_ => resolve_block_height(node, query, content_width, viewport),
	};
	// an explicit `height` overrides the resolved content height; a `min-height`
	// floors it (eg `100vh` to fill the terminal window).
	let content_height = box_model
		.height
		.unwrap_or(content_height)
		.max(box_model.min_height.unwrap_or(0));
	content_height + overhead.y
}

/// Block flow height: children stack, each laid out at the content width (less
/// any marker gutter), mirroring [`block_layout_rects`].
fn resolve_block_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
) -> u32 {
	let child_width = content_width.saturating_sub(marker_gutter(node, query));
	let children: Vec<_> = node.child_nodes(query).collect();
	let last = children.len().saturating_sub(1);
	children
		.iter()
		.enumerate()
		.map(|(i, child)| {
			let height =
				resolve_height(child, query, child_width, viewport).max(1);
			// the last child's trailing margin doesn't reserve a row (matches
			// `measure_block` and `block_layout_rects`).
			match i == last {
				true => {
					height.saturating_sub(node_bottom_margin(child, viewport))
				}
				false => height,
			}
		})
		.sum()
}

/// Flex height: form lines at the constrained main width, then sum line heights
/// (row) or take the tallest column (column), recursing through each item.
fn resolve_flex_height(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	content_width: u32,
	viewport: UVec2,
) -> u32 {
	let flexbox = node.flexbox();
	let mut child_sizes = node
		.child_nodes(query)
		.map(|child| (child.entity, child.intrinsic_size()))
		.collect::<Vec<_>>();
	child_sizes.sort_by_key(|(entity, _)| flex_order(*entity, query));
	let available = UVec2::new(content_width, viewport.y);
	let lines = form_lines(&child_sizes, flexbox, available, viewport);

	match resolve_direction(flexbox.direction, viewport) {
		// rows stack: total height is the sum of each line's tallest item
		Direction::Horizontal => lines
			.iter()
			.enumerate()
			.map(|(idx, line)| {
				let gap = if idx > 0 {
					flexbox.row_gap_cells(viewport)
				} else {
					0
				};
				let sizes = resolve_line_sizes(
					flexbox,
					line,
					query,
					content_width,
					// a row's cross axis is height, which is unbounded
					u32::MAX,
					viewport,
				);
				gap + sizes.iter().map(|size| size.y).max().unwrap_or(0)
			})
			.sum(),
		// columns sit side by side: height is the tallest column
		Direction::Vertical => lines
			.iter()
			.map(|line| {
				let gaps = flexbox.row_gap_cells(viewport)
					* (line.len().saturating_sub(1) as u32);
				let items: u32 = line
					.iter()
					.filter_map(|(entity, size)| {
						query.unresolved_node(*entity).ok().map(|child| {
							let width = size.x.min(content_width);
							resolve_height(&child, query, width, viewport)
						})
					})
					.sum();
				items + gaps
			})
			.max()
			.unwrap_or(0),
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	}
}

/// A line's flex-grown item sizes, with each item's height resolved at its
/// assigned (grown) width rather than the width it was measured at.
///
/// `container_cross` clamps the cross axis: in a column the cross axis is width,
/// which the fixed-width terminal bounds. An item measured at the unconstrained
/// viewport width would otherwise overflow the narrower column and, under
/// `align-items: center`, centre against a width larger than it actually has
/// (the homepage hero drifting past the sidebar). The main axis is left to the
/// flex-grow logic, so a row's heights stay unbounded (content scrolls).
fn resolve_line_sizes(
	flexbox: &FlexBox,
	line: &[(Entity, UVec2)],
	query: &CharcellQuery,
	container_main: u32,
	container_cross: u32,
	viewport: UVec2,
) -> Vec<UVec2> {
	let vertical =
		resolve_direction(flexbox.direction, viewport) == Direction::Vertical;
	apply_flex_grow(flexbox, line, query, container_main, viewport)
		.into_iter()
		.zip(line.iter())
		.map(|(size, (entity, _))| {
			// clamp the column cross size (width) to the container, then resolve
			// height at that assigned width so wrapped rows are fully reserved.
			let width = if vertical {
				size.x.min(container_cross)
			} else {
				size.x
			};
			let node = query.unresolved_node(*entity).ok();
			let content_height = node
				.as_ref()
				.map(|child| resolve_height(child, query, width, viewport))
				.unwrap_or(size.y);
			// A scroll container (overflow clipped on the main axis) keeps its
			// flex-grown size, so it clips and scrolls its own content instead of
			// growing the line and pushing later items (eg a chat composer) off
			// screen. A normal column item's height is the *main* axis, so keep the
			// larger of its flex-grown size and its content (a `flex-grow` item
			// filling the surplus, eg the page's sidebar+main row pushing the footer
			// to the bottom), never clipping wrapped content below it.
			let clips_main = node.as_ref().is_some_and(|child| {
				let overflow = if vertical {
					child.layout_style().overflow_y
				} else {
					child.layout_style().overflow_x
				};
				overflow.is_clipped()
			});
			// a row's height is the cross axis: take it from the content at the
			// assigned width.
			let height = match (vertical, clips_main) {
				(true, true) => size.y,
				(true, false) => size.y.max(content_height),
				(false, _) => content_height,
			};
			UVec2::new(width, height)
		})
		.collect()
}

/// Flex order of `entity`, defaulting to `0` for nodes without a layout style.
fn flex_order(entity: Entity, query: &CharcellQuery) -> i32 {
	query
		.unresolved_node(entity)
		.map(|node| node.layout_style().flex_order)
		.unwrap_or(0)
}

// ── Layout pass ───────────────────────────────────────────────────────────────

/// Layout pass: assign a [`LayoutRect`] to each flex child.
///
/// Reads pre-computed [`IntrinsicSize`] from the query and writes
/// each child's rect into `layout_rects`.
pub fn flex_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: IRect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, IRect>,
) -> Result {
	let flexbox = node.flexbox();

	let box_model = BoxModel::from_node(node, viewport);
	let content_rect =
		scrollport_of(node, query, box_model.content_rect(container_rect));
	// flex math works in unsigned cell sizes; the signed content rect origin is
	// re-added when each child rect is placed.
	let available = UVec2::new(
		content_rect.width().max(0) as u32,
		content_rect.height().max(0) as u32,
	);

	// Get child sizes directly from child_nodes (already computed by measure phase),
	// overriding either axis a child sizes explicitly (eg a percent `width`, which
	// the measure pass left content-sized) with that resolved size as its base.
	let mut child_sizes: Vec<(Entity, UVec2)> = node
		.flow_child_nodes(query)
		.map(|child| {
			let intrinsic = child.intrinsic_size();
			let (explicit_w, explicit_h) =
				explicit_box_size(&child, viewport, available);
			(
				child.entity,
				UVec2::new(
					explicit_w.unwrap_or(intrinsic.x),
					explicit_h.unwrap_or(intrinsic.y),
				),
			)
		})
		.collect();
	// Sort by flex_order
	child_sizes.sort_by_key(|(e, _)| flex_order(*e, query));
	let lines = form_lines(&child_sizes, flexbox, available, viewport);

	let direction = resolve_direction(flexbox.direction, viewport);
	let container_main = match direction {
		Direction::Horizontal => available.x,
		Direction::Vertical => available.y,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};
	let container_cross = match direction {
		Direction::Horizontal => available.y,
		Direction::Vertical => available.x,
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	// Final per-line item sizes: flex-grown along the main axis, with each
	// item's cross size (a row's heights) resolved at its assigned width so a
	// stretched line is tall enough for its wrapped content (see `resolve_height`).
	let final_per_line: Vec<Vec<UVec2>> = lines
		.iter()
		.map(|line| {
			resolve_line_sizes(
				flexbox,
				line,
				query,
				container_main,
				container_cross,
				viewport,
			)
		})
		.collect();
	let mut line_cross_sizes: Vec<u32> = final_per_line
		.iter()
		.map(|sizes| line_cross_size_for(sizes, flexbox.direction, viewport))
		.collect();
	// a single flex line fills the container's cross size (CSS: a single-line
	// container's line cross size is the container's inner cross size), so
	// `align-items: stretch` fills the container — the full-width header/footer and
	// the full-height sidebar rail — not just the content. Wrapped (multi-line)
	// flows keep their per-line content cross size, distributed by `align-content`.
	if let [only] = line_cross_sizes.as_mut_slice() {
		*only = (*only).max(container_cross);
	}

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
				let line_y =
					content_rect.min.y + line_positions[line_idx] as i32;
				let line_h = if flexbox.align_content == AlignContent::Stretch {
					let bonus = (container_cross
						- line_cross_sizes.iter().sum::<u32>()
						- if line_cross_sizes.len() > 1 {
							flexbox.row_gap_cells(viewport)
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

				let final_sizes = &final_per_line[line_idx];
				let main_positions = apply_justify(
					flexbox,
					line,
					final_sizes,
					available.x,
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
						+ cross_offset(entity, query, flexbox, child_h, line_h)
							as i32;
					let child_x =
						content_rect.min.x + main_positions[item_idx] as i32;
					let child_rect = IRect::new(
						child_x,
						child_y,
						child_x + fsize.x as i32,
						child_y + child_h as i32,
					);
					layout_rects.insert(entity, child_rect);
				}
			}
		}

		// ── Col layout ──────────────────────────────────────────────────────
		Direction::Vertical => {
			for (line_idx, line) in lines.iter().enumerate() {
				let line_x =
					content_rect.min.x + line_positions[line_idx] as i32;
				let line_w = if flexbox.align_content == AlignContent::Stretch {
					let bonus = (container_cross
						- line_cross_sizes.iter().sum::<u32>()
						- if line_cross_sizes.len() > 1 {
							flexbox.column_gap_cells(viewport)
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

				let final_sizes = &final_per_line[line_idx];
				let main_positions = apply_justify(
					flexbox,
					line,
					final_sizes,
					available.y,
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
						+ cross_offset(entity, query, flexbox, child_w, line_w)
							as i32;
					let child_y =
						content_rect.min.y + main_positions[item_idx] as i32;
					let child_rect = IRect::new(
						child_x,
						child_y,
						child_x + child_w as i32,
						child_y + fsize.y as i32,
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
		Direction::Horizontal => flexbox.column_gap_cells(viewport),
		Direction::Vertical => flexbox.row_gap_cells(viewport),
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
		.unresolved_node(entity)
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
		Direction::Horizontal => flexbox.column_gap_cells(viewport),
		Direction::Vertical => flexbox.row_gap_cells(viewport),
		_ => {
			unreachable!("resolve_direction should eliminate viewport variants")
		}
	};

	let gap_total = if line.len() > 1 {
		main_gap * (line.len() as u32 - 1)
	} else {
		0
	};

	// Collect flex_grow from each child's layout style
	let grow_values: Vec<u32> = line
		.iter()
		.map(|(entity, _)| {
			query
				.unresolved_node(*entity)
				.map(|n| n.layout_style().flex_grow)
				.unwrap_or(0)
		})
		.collect();

	let total_grow: u32 = grow_values.iter().sum();

	let natural_total: u32 = line
		.iter()
		.map(|(_, s)| main_size(*s, flexbox.direction, viewport))
		.sum();
	let non_grow_total: u32 = line
		.iter()
		.zip(grow_values.iter())
		.filter(|(_, grow)| **grow == 0)
		.map(|((_, s), _)| main_size(*s, flexbox.direction, viewport))
		.sum();

	// When the line has free space, growers take their natural size *plus* a
	// share of the surplus (standard flex-grow). When it overflows, growers
	// instead split the space left after the non-growing items, shrinking below
	// their natural size to fit (eg a text column beside a fixed sidebar) rather
	// than overflowing the line and clipping their content.
	let used = natural_total + gap_total;
	let resolve_main = |nat: u32, grow: u32| -> u32 {
		if total_grow == 0 || grow == 0 {
			return nat;
		}
		if used <= container_main {
			let surplus = container_main - used;
			nat + (surplus as u64 * grow as u64 / total_grow as u64) as u32
		} else {
			let grow_space =
				container_main.saturating_sub(non_grow_total + gap_total);
			(grow_space as u64 * grow as u64 / total_grow as u64) as u32
		}
	};

	line.iter()
		.zip(grow_values.iter())
		.map(|((_, nat), &grow)| {
			let main = resolve_main(
				main_size(*nat, flexbox.direction, viewport),
				grow,
			);
			match direction {
				Direction::Horizontal => UVec2::new(main, nat.y),
				Direction::Vertical => UVec2::new(nat.x, main),
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
		Direction::Horizontal => flexbox.column_gap_cells(viewport),
		Direction::Vertical => flexbox.row_gap_cells(viewport),
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
		Direction::Horizontal => flexbox.row_gap_cells(viewport),
		Direction::Vertical => flexbox.column_gap_cells(viewport),
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
		BoxStyle::default().with_border(Spacing::all(Length::Px(1.)))
	}

	/// A thick border (eg the blockquote callout's left rule) draws with the
	/// heavy box-drawing glyph; a thin one stays light.
	#[beet_core::test]
	fn thick_border_draws_heavy() {
		render((LayoutStyle::flex_row(), children![(
			rsx! {"Q"},
			BoxStyle::default()
				.with_border(Spacing {
					left: Length::Px(3.),
					..Spacing::DEFAULT
				})
				// padding gives the callout the height a border needs to draw
				.with_padding(Spacing::all(Length::Rem(1.)))
		)]))
		.xpect_contains("┃")
		.xnot()
		.xpect_contains("│");
	}

	#[beet_core::test]
	fn justify_start() {
		render((
			LayoutStyle::flex_row()
				.justify_content(JustifyContent::Start)
				.column_gap(Length::Rem(1.)),
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
				.column_gap(Length::Rem(1.)),
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
				.column_gap(Length::Rem(1.)),
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
		render((
			LayoutStyle::flex_row().column_gap(Length::Rem(3.)),
			children![(rsx! {"A"}, bordered()), (rsx! {"B"}, bordered()),],
		))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn flex_grow_distributes_space() {
		let output = render((
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! {"A"}, bordered()),
				(
					rsx! {"B"},
					bordered(),
					LayoutStyle::default().with_flex_grow(1)
				),
				(rsx! {"C"}, bordered()),
			],
		));
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
				.column_gap(Length::Rem(1.)),
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
				.column_gap(Length::Rem(1.)),
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
				.column_gap(Length::Rem(1.)),
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
		render(
			(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![
						(rsx! {"A"}, bordered()),
						(rsx! {"B"}, bordered()),
					],
					bordered()
				),
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![
						(rsx! {"C"}, bordered()),
						(rsx! {"D"}, bordered()),
					],
					bordered()
				),
			]),
		)
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn padding_with_content() {
		render((LayoutStyle::flex_row(), children![(
			rsx! {"X"},
			BoxStyle::default()
				.with_border(Spacing::all(Length::Px(1.)))
				.with_padding(Spacing::all(Length::Rem(0.5)))
		)]))
		.xpect_snapshot();
	}

	#[beet_core::test]
	fn column_with_multiple_items() {
		render(
			(LayoutStyle::flex_col().row_gap(Length::Rem(1.)), children![
				(rsx! {"First"}, bordered()),
				(rsx! {"Second"}, bordered()),
				(rsx! {"Third"}, bordered()),
			]),
		)
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
		let out = render((
			LayoutStyle::flex_col().row_gap(Length::Rem(1.)),
			children![
				(
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
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
					LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
					children![(rsx! { "Below" }, bordered(), VisualStyle {
						background: Some(Color::srgb(0.2, 0.5, 0.2)),
						..default()
					},),],
					bordered()
				),
			],
		));
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
		let out = render((
			LayoutStyle::flex_row().column_gap(Length::Rem(1.)),
			children![
				(rsx! { "中文" }, bordered()),
				(rsx! { "ＡＢＣ" }, bordered()),
			],
		));
		// "中文": 2 wide chars = 4 cols content → border top = ┌────┐
		out.as_str().xpect_contains("┌────┐");
		// "ＡＢＣ": 3 wide chars = 6 cols content → border top = ┌──────┐
		out.xpect_contains("┌──────┐");
	}
}
