use super::*;
use crate::style::StyledNodeView;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::URect;

/// Traverses the tree painting each node's box model and text content.
///
/// `parent` is `None` for the root node. Reads each node's position from
/// `layout_rects`, which must be populated by the layout phase first.
pub fn paint_tree(
	node: &StyledNodeView,
	layout_rects: &HashMap<Entity, URect>,
	parent: Option<(Entity, &VisualStyle)>,
	viewport: URect,
	buffer: &mut Buffer,
) -> Result {
	let Some(&layout_rect) = layout_rects.get(&node.entity) else {
		return Ok(());
	};
	paint_node(node, layout_rect, parent, viewport, buffer)?;
	let node_parent = (node.entity, node.visual_style());
	for child in &node.children {
		paint_tree(child, layout_rects, Some(node_parent), viewport, buffer)?;
	}
	Ok(())
}

fn paint_node(
	node: &StyledNodeView,
	layout_rect: URect,
	parent: Option<(Entity, &VisualStyle)>,
	viewport: URect,
	buffer: &mut Buffer,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let border_rect = box_model.border_rect(layout_rect);

	// 1. fill margin cells with the parent entity/style
	if let Some((parent_entity, parent_style)) = parent {
		if border_rect != layout_rect {
			draw_margin(
				buffer,
				layout_rect,
				border_rect,
				parent_style.clone(),
				parent_entity,
			);
		}
	}

	// 2. draw border if present
	if box_model.has_border {
		draw_border(buffer, border_rect, node);
	}

	// 3. fill padding cells with the current node entity/style
	let inner_rect = box_model.inner_rect(layout_rect);
	let content_rect = box_model.content_rect(layout_rect);
	if content_rect != inner_rect {
		draw_padding(
			buffer,
			inner_rect,
			content_rect,
			node.visual_style().clone(),
			node.entity,
		);
	}

	// 4. paint text content
	paint_text(node, content_rect, buffer)?;

	Ok(())
}
