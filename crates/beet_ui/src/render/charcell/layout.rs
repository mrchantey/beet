//! Layout phase: assign [`LayoutRect`] top-down (pre-order).
//!
//! Each node answers: *"Given the rect I've been granted, how do I distribute
//! space to my children?"*
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

use super::query::CharcellNodeData;
use super::query::CharcellQuery;

/// The definite screen rect a node occupies, including margin.
///
/// Written by the layout phase, read by the paint phase.
/// This is the single source of truth for where a node lives on screen.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutRect(pub URect);

/// ECS system: assign [`LayoutRect`] to all nodes top-down.
pub fn layout_nodes<B: Component + AsBuffer>(
	mut params: ParamSet<(CharcellQuery, Query<&mut LayoutRect>)>,
	tree: CharcellTree,
	roots: Populated<(Entity, &B)>,
) -> Result {
	for (root, buffer) in roots {
		let viewport_size = buffer.size();
		let ordered = tree.pre_order(root);

		// Root gets the full viewport rect
		let mut layout_rects = HashMap::<Entity, URect>::new();
		layout_rects
			.insert(root, URect::new(0, 0, viewport_size.x, viewport_size.y));

		// Read phase: use CharcellQuery to distribute rects to children
		{
			let charcell = params.p0();
			for &entity in &ordered {
				let Some(&node_rect) = layout_rects.get(&entity) else {
					continue;
				};
				let Ok(node) = charcell.unresolved_node(entity) else {
					continue;
				};

				match node.layout_style().display {
					Display::Flex => flex_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					Display::Block => block_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					Display::Inline => inline_layout_rects(
						&node,
						&charcell,
						node_rect,
						viewport_size,
						&mut layout_rects,
					)?,
					// removed from layout: skip the subtree so children get no
					// rects and are not drawn
					Display::None => {}
				}
			}
		}

		// Write phase: flush computed rects to ECS components
		for (entity, rect) in layout_rects {
			if let Ok(mut layout_rect) = params.p1().get_mut(entity) {
				layout_rect.set_if_neq(LayoutRect(rect));
			}
		}
	}
	Ok(())
}

/// Block flow: stack children top-to-bottom, each taking full parent width.
pub fn block_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	// a list item's marker occupies a left gutter; its children are inset past
	// it (the marker itself paints into the gutter in the paint phase).
	let gutter = marker_gutter(node, query);
	let child_min_x = (content_rect.min.x + gutter).min(content_rect.max.x);
	let child_width = content_rect.max.x.saturating_sub(child_min_x);
	let mut child_y = content_rect.min.y;
	for child in node.child_nodes(query) {
		if child_y >= content_rect.max.y {
			break;
		}
		// height resolved at the assigned width, not the wider measured width, so
		// a narrowed column reserves every wrapped row instead of clipping the tail.
		let child_height = resolve_height(&child, query, child_width, viewport);
		let child_rect = URect::new(
			child_min_x,
			child_y,
			content_rect.max.x,
			(child_y + child_height).min(content_rect.max.y),
		);
		layout_rects.insert(child.entity, child_rect);
		child_y += child_height.max(1);
	}
	Ok(())
}

/// Inline flow: place children left-to-right, wrapping rows when width is exceeded.
///
/// Each row's height equals the tallest child in that row.
pub fn inline_layout_rects(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	container_rect: URect,
	viewport: UVec2,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let max_width = content_rect.width();

	// Form rows: greedily pack children left-to-right, wrapping as needed
	let mut rows: Vec<Vec<(Entity, UVec2)>> = Vec::new();
	let mut current_row: Vec<(Entity, UVec2)> = Vec::new();
	let mut current_row_width = 0u32;

	for child in node.child_nodes(query) {
		let size = child.intrinsic_size();
		if !current_row.is_empty() && current_row_width + size.x > max_width {
			rows.push(std::mem::take(&mut current_row));
			current_row_width = 0;
		}
		current_row_width += size.x;
		current_row.push((child.entity, size));
	}
	if !current_row.is_empty() {
		rows.push(current_row);
	}

	// Assign rects: rows stack vertically, children in each row sit side-by-side
	let mut row_y = content_rect.min.y;
	for row in &rows {
		let row_height = row.iter().map(|(_, s)| s.y).max().unwrap_or(1);
		if row_y >= content_rect.max.y {
			break;
		}
		let mut child_x = content_rect.min.x;
		for &(entity, size) in row {
			let child_rect = URect::new(
				child_x,
				row_y,
				(child_x + size.x).min(content_rect.max.x),
				(row_y + size.y).min(content_rect.max.y),
			);
			layout_rects.insert(entity, child_rect);
			child_x += size.x;
		}
		row_y += row_height.max(1);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::*;

	fn render(bundle: impl Bundle) -> String {
		Buffer::render_oneshot_plain_sized(UVec2::new(20, 10), bundle)
			.trim_lines()
	}

	#[beet_core::test]
	fn inline_places_children_side_by_side() {
		let out = render((
			LayoutStyle {
				display: Display::Inline,
				..default()
			},
			children![rsx_direct!{"A"}, rsx_direct!{"B"}, rsx_direct!{"C"}],
		));
		// All three children should appear on the same line
		let first_line = out.lines().next().unwrap_or("");
		first_line.xpect_contains("A");
		first_line.xpect_contains("B");
		first_line.xpect_contains("C");
	}

	#[beet_core::test]
	fn inline_wraps_when_overflowing() {
		let out = Buffer::render_oneshot_plain_sized(
			UVec2::new(10, 10),
			(
				LayoutStyle {
					display: Display::Inline,
					..default()
				},
				children![rsx_direct!{"Hello"}, rsx_direct!{"World"}, rsx_direct!{"Foo"},],
			),
		)
		.trim_lines();
		// Should wrap — not all on one line since 5+5+3 = 13 > 10
		let lines: Vec<&str> = out.lines().collect();
		(lines.len() >= 2).xpect_true();
	}

	/// No rendered line may exceed the buffer width, otherwise the terminal
	/// soft-wraps and content appears a column too wide (see the `layout` example).
	#[beet_core::test]
	fn never_renders_wider_than_buffer() {
		let bordered =
			BoxStyle::default().with_border(Spacing::all(Length::Rem(1.)));
		let width = 12;
		// a grow box filling the line, an unbreakable run, and wide chars that
		// would straddle the right edge all stay within the buffer width.
		let bundle = (LayoutStyle::flex_row().column_gap(1), children![
			(rsx_direct!{ "中文日本語ＡＢＣ" }, bordered.clone()),
			(
				rsx_direct!{ "Supercalifragilistic" },
				bordered.clone(),
				LayoutStyle::default().with_flex_grow(1)
			),
		]);
		Buffer::render_oneshot_sized(UVec2::new(width, 6), bundle)
			.lines()
			.map(display_width)
			.all(|line_width| line_width <= width as usize)
			.xpect_true();
	}
}
