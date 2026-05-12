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


/// Inline flow: same algorithm as block layout.
///
/// Full inline (side-by-side) rendering is not yet implemented.
pub fn inline_layout_rects(
	node: &StyledNodeView,
	container_rect: URect,
	viewport: URect,
	intrinsic_sizes: &HashMap<Entity, UVec2>,
	layout_rects: &mut HashMap<Entity, URect>,
) -> Result {
	block_layout_rects(
		node,
		container_rect,
		viewport,
		intrinsic_sizes,
		layout_rects,
	)
}