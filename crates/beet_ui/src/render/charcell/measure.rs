//! Measure phase: compute [`IntrinsicSize`] bottom-up (post-order).
//!
//! Each node answers: *"If I had infinite space, how big would I want to be?"*
use super::*;
use crate::style::Display;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// The node's preferred size before parent constraints apply.
///
/// Written by the measure phase, read by the layout phase.
#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub struct IntrinsicSize(pub UVec2);

/// ECS system: compute [`IntrinsicSize`] for all nodes bottom-up.
pub fn measure_nodes(
	mut params: ParamSet<(CharcellQuery, Query<&mut IntrinsicSize>)>,
	children_query: Query<&Children>,
	roots: Query<(Entity, &DoubleBuffer)>,
) -> Result {
	for (root, buffer) in roots {
		let viewport_size = buffer.size();
		let ordered = children_query.collect_post_order(root);
		let mut sizes = HashMap::<Entity, UVec2>::new();

		// Read phase: use CharcellQuery to measure each node bottom-up
		{
			let charcell = params.p0();
			for &entity in &ordered {
				let Ok(node) = charcell.node(entity) else {
					continue;
				};
				let size =
					measure_node(&node, &charcell, viewport_size, &sizes)?;
				sizes.insert(entity, size);
			}
		}

		// Write phase: flush computed sizes to ECS components
		for (entity, size) in sizes {
			if let Ok(mut intrinsic) = params.p1().get_mut(entity) {
				intrinsic.set_if_neq(IntrinsicSize(size));
			}
		}
	}
	Ok(())
}

/// Compute a single node's intrinsic size.
///
/// Uses viewport dimensions as the unconstrained available space.
pub(super) fn measure_node(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_available = UVec2::new(
		viewport.x.saturating_sub(overhead.x),
		viewport.y.saturating_sub(overhead.y),
	);
	let content_size = match node.layout_style().display {
		Display::Flex => {
			measure_flex(node, query, sizes, content_available, viewport)?
		}
		Display::Inline => {
			measure_inline(node, query, content_available, sizes)?
		}
		_ if node.value().is_some() => measure_text(node, content_available.x),
		_ => UVec2::ZERO,
	};
	(content_size + overhead).xok()
}

/// Measure inline container: width = max row width, height = sum of row heights.
///
/// Children must already be measured (post-order traversal ensures this).
fn measure_inline(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	available: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	let mut max_w = 0u32;
	let mut total_h = 0u32;
	let mut row_w = 0u32;
	let mut row_h = 0u32;

	let mut children = node.child_nodes(query).peekable();
	if children.peek().is_some() {
		return UVec2::ZERO.xok();
	}
	for child in children {
		// Use freshly-computed sizes during this measure pass
		let size = sizes
			.get(&child.entity)
			.copied()
			.unwrap_or_else(|| child.intrinsic_size());
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
