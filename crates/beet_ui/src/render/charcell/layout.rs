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
pub fn layout_nodes(
	mut params: ParamSet<(CharcellQuery, Query<&mut LayoutRect>)>,
	children: Query<&Children>,
	roots: Query<(Entity, &DoubleBuffer)>,
) -> Result {
	for (root, buffer) in roots {
		let viewport_size = buffer.size();
		let ordered = children.collect_pre_order(root);

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
				let Ok(node) = charcell.node(entity) else {
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
	let mut child_y = content_rect.min.y;
	for child in node.child_nodes(query) {
		if child_y >= content_rect.max.y {
			break;
		}
		let child_size = child.intrinsic_size();
		let child_rect = URect::new(
			content_rect.min.x,
			child_y,
			content_rect.max.x,
			(child_y + child_size.y).min(content_rect.max.y),
		);
		layout_rects.insert(child.entity, child_rect);
		child_y += child_size.y.max(1);
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
		CharcellPlugin::render_oneshot_plain_sized(UVec2::new(20, 10), bundle)
			.trim_lines()
	}

	#[test]
	fn inline_places_children_side_by_side() {
		let out = render((
			LayoutStyle {
				display: Display::Inline,
				..default()
			},
			children![rsx! {"A"}, rsx! {"B"}, rsx! {"C"}],
		));
		// All three children should appear on the same line
		let first_line = out.lines().next().unwrap_or("");
		first_line.xpect_contains("A");
		first_line.xpect_contains("B");
		first_line.xpect_contains("C");
	}

	#[test]
	fn inline_wraps_when_overflowing() {
		let out = CharcellPlugin::render_oneshot_plain_sized(
			UVec2::new(10, 10),
			(
				LayoutStyle {
					display: Display::Inline,
					..default()
				},
				children![rsx! {"Hello"}, rsx! {"World"}, rsx! {"Foo"},],
			),
		)
		.trim_lines();
		// Should wrap — not all on one line since 5+5+3 = 13 > 10
		let lines: Vec<&str> = out.lines().collect();
		(lines.len() >= 2).xpect_true();
	}
}
