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
	let size = measure_node(node, viewport)?;
	sizes.insert(node.entity, size);
	Ok(())
}

/// Compute a single node's intrinsic size.
///
/// Uses viewport dimensions as the unconstrained available space, per the
/// measure phase contract: no parent narrowing, viewport is the only global constraint.
fn measure_node(node: &StyledNodeView, viewport: URect) -> Result<UVec2> {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_available = UVec2::new(
		viewport.width().saturating_sub(overhead.x),
		viewport.height().saturating_sub(overhead.y),
	);
	let content_size = match node.layout_style().display {
		Display::Flex => flex_measure(node, content_available, viewport)?,
		_ if node.value.is_some() => measure_text(node, content_available.x),
		_ => UVec2::ZERO,
	};
	(content_size + overhead).xok()
}
