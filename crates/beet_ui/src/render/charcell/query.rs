//! Core query types for the charcell render pipeline.
//!
//! Provides [`CharcellNodeData`] (a flat per-node style view without children),
//! [`CharcellQuery`] (the shared system parameter), [`StylesQuery`] (the style
//! component query alias), and ECS tree traversal helpers.
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

use super::DoubleBuffer;

/// Type alias for the style component query used throughout the render pipeline.
pub(super) type StylesQuery<'w, 's> = Query<
	'w,
	's,
	(
		Option<&'static Value>,
		Option<&'static VisualStyle>,
		Option<&'static LayoutStyle>,
		Option<&'static BoxStyle>,
	),
>;

/// Per-node flat style view, without children.
///
/// Children are traversed separately via ECS [`Children`] queries.
/// A new instance is created per node during each render phase.
pub(super) struct CharcellNodeData<'a> {
	pub entity: Entity,
	pub value: Option<&'a Value>,
	pub visual: Option<&'a VisualStyle>,
	pub layout: Option<&'a LayoutStyle>,
	pub box_style: Option<&'a BoxStyle>,
}

impl CharcellNodeData<'_> {
	/// Resolved visual style, defaulting to [`VISUAL_STYLE_DEFAULT`].
	pub fn visual_style(&self) -> &VisualStyle {
		self.visual.unwrap_or(&VISUAL_STYLE_DEFAULT)
	}
	/// Resolved layout style, defaulting to [`LAYOUT_STYLE_DEFAULT`].
	pub fn layout_style(&self) -> &LayoutStyle {
		self.layout.unwrap_or(&LAYOUT_STYLE_DEFAULT)
	}
	/// Flexbox config from the layout style.
	pub fn flexbox(&self) -> &FlexBox { &self.layout_style().flex_box }
}

/// System parameter shared by all charcell render systems.
#[derive(SystemParam)]
pub struct CharcellQuery<'w, 's> {
	pub roots: Query<'w, 's, (Entity, &'static DoubleBuffer)>,
	pub styles: StylesQuery<'w, 's>,
	pub children: Query<'w, 's, &'static Children>,
}

impl CharcellQuery<'_, '_> {
	/// Collect `(root_entity, viewport_size)` for all [`DoubleBuffer`] roots.
	pub fn root_viewports(&self) -> Vec<(Entity, UVec2)> {
		self.roots
			.iter()
			.map(|(entity, buffer)| (entity, buffer.size()))
			.collect()
	}
}

/// Collect entities in post-order (leaves first) starting from `root`.
pub(super) fn collect_post_order(
	root: Entity,
	children: &Query<&Children>,
) -> Vec<Entity> {
	let mut result = Vec::new();
	post_order_visit(root, children, &mut result);
	result
}

fn post_order_visit(
	entity: Entity,
	children: &Query<&Children>,
	result: &mut Vec<Entity>,
) {
	if let Ok(children_list) = children.get(entity) {
		for child in children_list.iter() {
			post_order_visit(child, children, result);
		}
	}
	result.push(entity);
}

/// Collect entities in pre-order (root first) starting from `root`.
pub(super) fn collect_pre_order(
	root: Entity,
	children: &Query<&Children>,
) -> Vec<Entity> {
	let mut result = Vec::new();
	pre_order_visit(root, children, &mut result);
	result
}

fn pre_order_visit(
	entity: Entity,
	children: &Query<&Children>,
	result: &mut Vec<Entity>,
) {
	result.push(entity);
	if let Ok(children_list) = children.get(entity) {
		for child in children_list.iter() {
			pre_order_visit(child, children, result);
		}
	}
}
