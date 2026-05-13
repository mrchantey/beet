use super::*;

use beet_core::prelude::*;
use bevy::math::UVec2;

/// ECS system: paint all nodes in each [`DoubleBuffer`] tree.
///
/// Traverses each tree pre-order. Each node fills its background (if set),
/// draws its border, then paints text. Pre-order ensures parents fill first so
/// children naturally overlay their margin area without any parent lookup.
pub fn paint_nodes(
	mut roots: Query<(Entity, &mut DoubleBuffer)>,
	charcell: CharcellQuery,
	children_query: Query<&Children>,
) -> Result {
	for (root, mut buffer) in roots.iter_mut() {
		let viewport_size = buffer.size();
		let ordered = children_query.collect_pre_order(root);

		// full reset may become a problematic pattern if we want to do
		// partial paints
		buffer.current_buffer_mut().reset();
		let buf = buffer.current_buffer_mut();

		for &entity in &ordered {
			let Ok(node) = charcell.node(entity) else {
				continue;
			};
			paint_node(&node, viewport_size, buf)?;
		}
	}
	Ok(())
}

fn paint_node(
	node: &CharcellNodeData,
	viewport: UVec2,
	buffer: &mut Buffer,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let layout_rect = node.layout_rect();
	let border_rect = box_model.border_rect(layout_rect);
	let inner_rect = box_model.inner_rect(layout_rect);
	let content_rect = box_model.content_rect(layout_rect);

	// 1. Fill inner rect with background only when the node has a background color.
	//    Skipping transparent nodes keeps "empty" rows as Cell::BLANK so
	//    trim_lines can strip trailing blank rows correctly.
	if node.visual_style().background.is_some()
		&& inner_rect.width() > 0
		&& inner_rect.height() > 0
	{
		buffer.fill_rect(
			inner_rect,
			Cell::new(" ", node.visual_style().clone(), node.entity),
		);
	}

	// 2. Draw border if present
	if box_model.has_border {
		draw_border(buffer, border_rect, node);
	}

	// 3. Paint text content
	// this is a no-op if no Value
	paint_text(node, content_rect, buffer)?;

	Ok(())
}
