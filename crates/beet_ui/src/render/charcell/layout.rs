//! Layout phase: assign [`LayoutRect`] top-down (pre-order).
//!
//! Each node answers: *"Given the rect I've been granted, how do I distribute
//! space to my children?"*
use super::*;
use crate::style::Display;
use crate::style::StyledNodeView;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// The definite screen rect a node occupies, including margin.
///
/// Written by the layout phase, read by the paint phase.
/// This is the single source of truth for where a node lives on screen.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct LayoutRect(pub URect);

/// Traverses the subtree pre-order, assigning a [`LayoutRect`] to each child.
///
/// `node_rect` is the outer rect already assigned to `node`.
/// Children are routed to [`flex_layout_rects`] or [`block_layout_rects`]
/// based on the node's [`Display`] property, then recursed into.
pub fn layout_tree(
	node: &StyledNodeView,
	node_rect: URect,
	viewport: URect,
	intrinsic_sizes: &HashMap<Entity, UVec2>,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	match node.layout_style().display {
		Display::Flex => flex_layout_rects(
			node,
			node_rect,
			viewport,
			intrinsic_sizes,
			layout_rects,
		)?,
		Display::Block => block_layout_rects(
			node,
			node_rect,
			viewport,
			intrinsic_sizes,
			layout_rects,
		)?,
		Display::Inline => inline_layout_rects(
			node,
			node_rect,
			viewport,
			intrinsic_sizes,
			layout_rects,
		)?,
	}
	for child in &node.children {
		if let Some(&child_rect) = layout_rects.get(&child.entity) {
			layout_tree(
				child,
				child_rect,
				viewport,
				intrinsic_sizes,
				layout_rects,
			)?;
		}
	}
	Ok(())
}

/// Block flow: stack children top-to-bottom, each taking full parent width.
pub fn block_layout_rects(
	node: &StyledNodeView,
	container_rect: URect,
	viewport: URect,
	intrinsic_sizes: &HashMap<Entity, UVec2>,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	if node.children.is_empty() {
		return Ok(());
	}
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let mut child_y = content_rect.min.y;
	for child in &node.children {
		if child_y >= content_rect.max.y {
			break;
		}
		let child_size = intrinsic_sizes
			.get(&child.entity)
			.copied()
			.unwrap_or_default();
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
	node: &StyledNodeView,
	container_rect: URect,
	viewport: URect,
	intrinsic_sizes: &HashMap<Entity, UVec2>,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	if node.children.is_empty() {
		return Ok(());
	}
	let box_model = BoxModel::from_node(node, viewport);
	let content_rect = box_model.content_rect(container_rect);
	let max_width = content_rect.width();

	// Form rows: greedily pack children left-to-right, wrapping as needed
	let mut rows: Vec<Vec<(&StyledNodeView, UVec2)>> = Vec::new();
	let mut current_row: Vec<(&StyledNodeView, UVec2)> = Vec::new();
	let mut current_row_width = 0u32;

	for child in &node.children {
		let size = intrinsic_sizes
			.get(&child.entity)
			.copied()
			.unwrap_or_default();
		if !current_row.is_empty() && current_row_width + size.x > max_width {
			rows.push(std::mem::take(&mut current_row));
			current_row_width = 0;
		}
		current_row_width += size.x;
		current_row.push((child, size));
	}
	if !current_row.is_empty() {
		rows.push(current_row);
	}

	// Assign rects: each row stacks vertically, children in each row sit side-by-side
	let mut row_y = content_rect.min.y;
	for row in &rows {
		let row_height = row.iter().map(|(_, s)| s.y).max().unwrap_or(1);
		if row_y >= content_rect.max.y {
			break;
		}
		let mut child_x = content_rect.min.x;
		for (child, size) in row {
			let child_rect = URect::new(
				child_x,
				row_y,
				(child_x + size.x).min(content_rect.max.x),
				(row_y + size.y).min(content_rect.max.y),
			);
			layout_rects.insert(child.entity, child_rect);
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
		CharcellRenderer::new_size(20, 10)
			.render_oneshot(bundle)
			.unwrap()
			.render_plain()
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
		// Use a narrow viewport (10 wide) with wide children
		let out = CharcellRenderer::new_size(10, 10)
			.render_oneshot((
				LayoutStyle {
					display: Display::Inline,
					..default()
				},
				children![rsx! {"Hello"}, rsx! {"World"}, rsx! {"Foo"},],
			))
			.unwrap()
			.render_plain()
			.trim_lines();
		// Should wrap — not all on one line since 5+5+3 = 13 > 10
		let lines: Vec<&str> = out.lines().collect();
		(lines.len() >= 2).xpect_true();
	}
}
