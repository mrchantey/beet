use super::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// Snapshot of per-node paint data, cloned before the buffer is mutated.
///
/// All data is owned so the read and write phases of [`paint_nodes`] can
/// operate independently without holding a borrow on [`CharcellQuery`].
struct NodePaint {
	entity: Entity,
	layout_rect: URect,
	visual: VisualStyle,
	box_style: Option<BoxStyle>,
	value: Option<Value>,
}

/// ECS system: paint all nodes in each [`DoubleBuffer`] tree.
///
/// Traverses each tree pre-order. Each node fills its own inner rect with its
/// background, draws its border, then paints text. Since rendering is pre-order,
/// a parent's fill naturally covers the child's margin area — no parent lookup
/// is needed in the child.
pub fn paint_nodes(
	mut params: ParamSet<(Query<(Entity, &mut DoubleBuffer)>, CharcellQuery)>,
	children_query: Query<&Children>,
) -> Result {
	let root_viewports = params.p1().root_viewports();

	for (root, viewport_size) in root_viewports {
		let ordered = children_query.collect_pre_order(root);

		// Read phase: snapshot node data from CharcellQuery
		let paint_items: Vec<NodePaint> = {
			let charcell = params.p1();
			ordered
				.iter()
				.filter_map(|&entity| {
					let node = charcell.node(entity).ok()?;
					Some(NodePaint {
						entity,
						layout_rect: node.layout_rect(),
						visual: node.visual_style().clone(),
						box_style: node.box_style().cloned(),
						value: node.value().cloned(),
					})
				})
				.collect()
		};

		// Write phase: reset and paint into the buffer
		let mut buffers = params.p0();
		let Ok((_, mut double_buffer)) = buffers.get_mut(root) else {
			continue;
		};
		double_buffer.current_buffer_mut().reset();
		let buf = double_buffer.current_buffer_mut();

		for item in &paint_items {
			paint_node(item, viewport_size, buf)?;
		}
	}
	Ok(())
}

fn paint_node(
	item: &NodePaint,
	viewport: UVec2,
	buffer: &mut Buffer,
) -> Result {
	let box_model = BoxModel::from_box_style(item.box_style.as_ref(), viewport);
	let layout_rect = item.layout_rect;
	let border_rect = box_model.border_rect(layout_rect);
	let inner_rect = box_model.inner_rect(layout_rect);
	let content_rect = box_model.content_rect(layout_rect);

	// 1. Fill the inner rect (inside margin+border) with this node's background.
	//    Pre-order traversal means the parent fills first, so a child's margin
	//    area is covered by the parent's fill without any parent lookup.
	if inner_rect.width() > 0 && inner_rect.height() > 0 {
		buffer.fill_rect(inner_rect, item.visual.clone(), item.entity);
	}

	// 2. Draw border if present
	if box_model.has_border {
		draw_border(
			buffer,
			border_rect,
			item.box_style.as_ref(),
			&item.visual,
			item.entity,
		);
	}

	// 3. Paint text content
	if let Some(ref value) = item.value {
		paint_text_raw(
			&value.to_string(),
			&item.visual,
			item.entity,
			content_rect,
			buffer,
		)?;
	}

	Ok(())
}
