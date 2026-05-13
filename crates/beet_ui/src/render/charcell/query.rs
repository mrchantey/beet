//! Core query types for the charcell render pipeline.
//!
//! Provides [`CharcellNodeData`] (a flat per-node style view without children),
//! and [`CharcellQuery`] (the shared system parameter).

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::UVec2;

/// Per-node flat style view, without children.
///
/// Children are traversed separately via ECS [`Children`] queries.
/// A new instance is created per node during each render phase.
pub(super) struct CharcellNodeData<'a> {
	pub entity: Entity,
	intrinsic_size: &'a IntrinsicSize,
	layout_rect: &'a LayoutRect,
	value: Option<&'a Value>,
	visual: Option<&'a VisualStyle>,
	layout: Option<&'a LayoutStyle>,
	children: Option<&'a Children>,
	box_style: Option<&'a BoxStyle>,
}

impl CharcellNodeData<'_> {
	pub fn value(&self) -> Option<&Value> { self.value }

	pub fn intrinsic_size(&self) -> UVec2 { self.intrinsic_size.0 }
	pub fn layout_rect(&self) -> URect { self.layout_rect.0 }

	/// return option here, allows for computation skips
	pub fn box_style(&self) -> Option<&BoxStyle> { self.box_style }
	/// Resolved layout style, defaulting to [`LAYOUT_STYLE_DEFAULT`].
	pub fn layout_style(&self) -> &LayoutStyle {
		self.layout.unwrap_or(&LAYOUT_STYLE_DEFAULT)
	}

	/// Resolved visual style, defaulting to [`VISUAL_STYLE_DEFAULT`].
	pub fn visual_style(&self) -> &VisualStyle {
		self.visual.unwrap_or(&VISUAL_STYLE_DEFAULT)
	}
	/// Flexbox config from the layout style.
	pub fn flexbox(&self) -> &FlexBox { &self.layout_style().flex_box }

	pub(super) fn child_nodes<'a>(
		&'a self,
		query: &'a CharcellQuery,
	) -> impl 'a + Iterator<Item = CharcellNodeData<'a>> {
		self.children
			.iter()
			.flat_map(|children| children.iter())
			.filter_map(move |child| query.node(child).ok())
	}
}

/// System parameter shared by all charcell render systems.
#[derive(SystemParam)]
pub struct CharcellQuery<'w, 's> {
	nodes: Query<
		'w,
		's,
		(
			&'static IntrinsicSize,
			&'static LayoutRect,
			Option<&'static Value>,
			Option<&'static VisualStyle>,
			Option<&'static LayoutStyle>,
			Option<&'static BoxStyle>,
			Option<&'static Children>,
		),
	>,
}

impl CharcellQuery<'_, '_> {
	pub(super) fn node(&self, entity: Entity) -> Result<CharcellNodeData<'_>> {
		let (
			intrinsic_size,
			layout_rect,
			value,
			visual,
			layout,
			box_style,
			children,
		) = self.nodes.get(entity)?;
		Ok(CharcellNodeData {
			intrinsic_size,
			layout_rect,
			entity,
			value,
			visual,
			layout,
			children,
			box_style,
		})
	}
}
