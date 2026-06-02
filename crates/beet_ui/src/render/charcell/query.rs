//! Core query types for the charcell render pipeline.
//!
//! Provides [`CharcellNodeData`] (a flat per-node style view without children),
//! and [`CharcellQuery`] (the shared system parameter).

use crate::prelude::*;
use crate::style::Display;
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
	hyperlink: Option<&'a Hyperlink>,
	marker: Option<&'a Marker>,
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

	/// OSC-8 hyperlink target, if this element is an `<a>`/`<img>`.
	pub fn hyperlink(&self) -> Option<&str> {
		self.hyperlink.map(|link| link.0.as_str())
	}

	/// Generated leading content (bullet, quote bar, rule, alt text), if any.
	pub fn marker(&self) -> Option<&str> {
		self.marker.map(|marker| marker.0.as_str())
	}
	/// Flexbox config from the layout style.
	pub fn flexbox(&self) -> &FlexBox { &self.layout_style().flex_box }

	/// Whether this node is inline-level content: a text [`Value`] leaf or an
	/// element with `display: inline`. Inline-level children cause their
	/// container to establish an inline formatting context.
	pub fn is_inline_level(&self) -> bool {
		self.value().is_some() || self.layout_style().display == Display::Inline
	}

	pub(super) fn child_nodes<'a>(
		&'a self,
		query: &'a CharcellQuery,
	) -> impl 'a + Iterator<Item = CharcellNodeData<'a>> {
		// `display: none` children are removed from layout: skipping them here
		// means they reserve no space (measure) and are never assigned a rect
		// (layout), so the subtree is neither sized nor painted.
		self.children
			.iter()
			.flat_map(|children| children.iter())
			.filter_map(move |child| query.node(child).ok())
			.filter(|node| node.layout_style().display != Display::None)
	}

	/// Whether this node has any renderable child nodes.
	pub(super) fn has_child_nodes(&self, query: &CharcellQuery) -> bool {
		self.child_nodes(query).next().is_some()
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
			Option<&'static Hyperlink>,
			Option<&'static Marker>,
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
			hyperlink,
			marker,
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
			hyperlink,
			marker,
		})
	}
}
