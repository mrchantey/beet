use super::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;
use bevy::prelude::ChildOf;

/// ECS system: paint all nodes in each [`DoubleBuffer`] tree.
///
/// Traverses each tree pre-order, painting box model and text for every
/// node that has a [`LayoutRect`]. Parent visual style is used to fill
/// margin cells.
pub fn paint_nodes(
	mut buffers: Query<(Entity, &mut DoubleBuffer)>,
	children_query: Query<&Children>,
	styles_query: StylesQuery,
	layout_rects: Query<&LayoutRect>,
	parent_query: Query<&ChildOf>,
) -> Result {
	for (root, mut double_buffer) in buffers.iter_mut() {
		// Reset the current frame buffer, then collect the ordered node list
		double_buffer.current_buffer_mut().reset();
		let viewport_size = double_buffer.size();
		let ordered = collect_pre_order(root, &children_query);

		// Paint each node into the buffer
		let buffer = double_buffer.current_buffer_mut();
		for &entity in &ordered {
			let Ok(&layout_rect) = layout_rects.get(entity) else {
				continue;
			};

			// Look up parent visual style for margin filling
			let parent_data: Option<(Entity, &VisualStyle)> =
				parent_query.get(entity).ok().and_then(|child_of| {
					let pe = child_of.parent();
					let (_, pvisual, _, _) = styles_query.get(pe).ok()?;
					Some((pe, pvisual.unwrap_or(&VISUAL_STYLE_DEFAULT)))
				});

			let (value, visual, layout, box_style) =
				styles_query.get(entity).unwrap_or_default();
			let node = CharcellNodeData {
				entity,
				value,
				visual,
				layout,
				box_style,
			};

			paint_node(
				&node,
				layout_rect.0,
				parent_data,
				viewport_size,
				buffer,
			)?;
		}
	}
	Ok(())
}

fn paint_node(
	node: &CharcellNodeData,
	layout_rect: URect,
	parent: Option<(Entity, &VisualStyle)>,
	viewport: UVec2,
	buffer: &mut Buffer,
) -> Result {
	let box_model = BoxModel::from_node(node, viewport);
	let border_rect = box_model.border_rect(layout_rect);

	// 1. Fill margin cells with the parent entity/style
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

	// 2. Draw border if present
	if box_model.has_border {
		draw_border(buffer, border_rect, node);
	}

	// 3. Fill padding cells with the current node entity/style
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

	// 4. Paint text content
	paint_text(node, content_rect, buffer)?;

	Ok(())
}
