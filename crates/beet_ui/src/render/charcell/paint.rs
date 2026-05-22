use super::*;

use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::math::UVec2;

/// ECS system: paint all nodes in each [`DoubleBuffer`] tree.
///
/// Traverses each tree pre-order. Each node fills its background (if set),
/// draws its border, then paints text. Pre-order ensures parents fill first so
/// children naturally overlay their margin area without any parent lookup.
///
/// Nodes inside an [inline formatting context](inline) are painted by their
/// container, so the whole subtree below an IFC owner is skipped here.
pub fn paint_nodes<B: Component<Mutability = Mutable> + AsBuffer>(
	mut roots: Populated<(Entity, &mut B)>,
	charcell: CharcellQuery,
	children_query: Query<&Children>,
) -> Result {
	for (root, mut buffer) in roots.iter_mut() {
		let viewport_size = buffer.size();
		let ordered = children_query.collect_pre_order(root);

		// descendants of an IFC owner are painted by the owner, not themselves
		let mut managed = HashSet::<Entity>::default();
		for &entity in &ordered {
			if managed.contains(&entity) {
				continue;
			}
			let Ok(node) = charcell.node(entity) else {
				continue;
			};
			if establishes_inline_flow(&node, &charcell) {
				managed.extend(children_query.iter_descendants(entity));
			}
		}

		// full reset may become a problematic pattern if we want to do
		// partial paints
		buffer.reset();

		for &entity in &ordered {
			if managed.contains(&entity) {
				continue;
			}
			let Ok(node) = charcell.node(entity) else {
				continue;
			};
			paint_node(&node, &charcell, viewport_size, &mut *buffer)?;
		}
	}
	Ok(())
}

fn paint_node(
	node: &CharcellNodeData,
	query: &CharcellQuery,
	viewport: UVec2,
	buffer: &mut impl AsBuffer,
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
		draw_border(&mut *buffer, border_rect, node);
	}

	// 3. Paint content: flow inline descendants if this owns an inline
	//    formatting context, otherwise paint this node's own text (a no-op
	//    when it has no value).
	if establishes_inline_flow(node, query) {
		paint_inline_flow(node, query, content_rect, &mut *buffer);
	} else {
		paint_text(node, content_rect, &mut *buffer)?;
	}

	Ok(())
}
