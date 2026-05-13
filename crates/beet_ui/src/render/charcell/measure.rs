//! Measure phase: compute [`IntrinsicSize`] bottom-up (post-order).
//!
//! Each node answers: *"If I had infinite space, how big would I want to be?"*
use super::*;
use crate::style::Display;
use crate::style::StyledNodeView;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// The node's preferred size before parent constraints apply.
///
/// Written by the measure phase, read by the layout phase.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct IntrinsicSize(pub UVec2);

/// Traverses the subtree post-order, storing each node's intrinsic size in `sizes`.
pub fn measure_tree(
	node: &StyledNodeView,
	viewport: URect,
	sizes: &mut HashMap<Entity, UVec2>,
) -> Result {
	for child in &node.children {
		measure_tree(child, viewport, sizes)?;
	}
	let size = measure_node(node, viewport, sizes)?;
	sizes.insert(node.entity, size);
	Ok(())
}

/// Compute a single node's intrinsic size.
///
/// Uses viewport dimensions as the unconstrained available space, per the
/// measure phase contract: no parent narrowing, viewport is the only global constraint.
fn measure_node(
	node: &StyledNodeView,
	viewport: URect,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_available = UVec2::new(
		viewport.width().saturating_sub(overhead.x),
		viewport.height().saturating_sub(overhead.y),
	);
	let content_size = match node.layout_style().display {
		Display::Flex => measure_flex(node, content_available, viewport)?,
		Display::Inline => measure_inline(node, content_available, sizes)?,
		_ if node.value.is_some() => measure_text(node, content_available.x),
		_ => UVec2::ZERO,
	};
	(content_size + overhead).xok()
}

/// Measure inline container: width = sum of child widths (first-row), height = max child height.
///
/// Children must already be measured (post-order traversal ensures this).
fn measure_inline(
	node: &StyledNodeView,
	available: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	if node.children.is_empty() {
		return UVec2::ZERO.xok();
	}
	let mut max_w = 0u32;
	let mut total_h = 0u32;
	let mut row_w = 0u32;
	let mut row_h = 0u32;

	for child in &node.children {
		let size = sizes.get(&child.entity).copied().unwrap_or_default();
		if row_w > 0 && row_w + size.x > available.x {
			max_w = max_w.max(row_w);
			total_h += row_h.max(1);
			row_w = 0;
			row_h = 0;
		}
		row_w += size.x;
		row_h = row_h.max(size.y);
	}
	max_w = max_w.max(row_w);
	total_h += row_h.max(1);
	UVec2::new(max_w, total_h).xok()
}
