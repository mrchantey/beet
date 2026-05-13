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
	query: CharcellQuery,
	mut sizes_query: Query<&mut IntrinsicSize>,
) -> Result {
	let root_viewports = query.root_viewports();
	for (root, viewport_size) in root_viewports {
		let ordered = collect_post_order(root, &query.children);
		let mut sizes = HashMap::<Entity, UVec2>::new();

		for &entity in &ordered {
			// Access fields directly to allow partial borrows alongside sizes HashMap
			let (value, visual, layout, box_style) =
				query.styles.get(entity).unwrap_or_default();
			let node = CharcellNodeData {
				entity,
				value,
				visual,
				layout,
				box_style,
			};

			let children: Vec<Entity> = query
				.children
				.get(entity)
				.map(|c| c.iter().collect())
				.unwrap_or_default();

			let size = measure_node(
				&node,
				&children,
				viewport_size,
				&sizes,
				&query.styles,
			)?;
			sizes.insert(entity, size);
		}

		// Write sizes to ECS components
		for (entity, size) in sizes {
			if let Ok(mut intrinsic) = sizes_query.get_mut(entity) {
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
	children: &[Entity],
	viewport: UVec2,
	sizes: &HashMap<Entity, UVec2>,
	styles: &StylesQuery<'_, '_>,
) -> Result<UVec2> {
	let box_model = BoxModel::from_node(node, viewport);
	let overhead = box_model.overhead();
	let content_available = UVec2::new(
		viewport.x.saturating_sub(overhead.x),
		viewport.y.saturating_sub(overhead.y),
	);
	let content_size = match node.layout_style().display {
		Display::Flex => measure_flex(
			node,
			children,
			sizes,
			styles,
			content_available,
			viewport,
		)?,
		Display::Inline => measure_inline(children, content_available, sizes)?,
		_ if node.value.is_some() => measure_text(node, content_available.x),
		_ => UVec2::ZERO,
	};
	(content_size + overhead).xok()
}

/// Measure inline container: width = max row width, height = sum of row heights.
///
/// Children must already be measured (post-order traversal ensures this).
fn measure_inline(
	children: &[Entity],
	available: UVec2,
	sizes: &HashMap<Entity, UVec2>,
) -> Result<UVec2> {
	if children.is_empty() {
		return UVec2::ZERO.xok();
	}
	let mut max_w = 0u32;
	let mut total_h = 0u32;
	let mut row_w = 0u32;
	let mut row_h = 0u32;

	for &entity in children {
		let size = sizes.get(&entity).copied().unwrap_or_default();
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
