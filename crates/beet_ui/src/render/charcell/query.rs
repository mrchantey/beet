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
		// (layout), so the subtree is neither sized nor painted. A child that is a
		// [`RenderRef`] holder is resolved to the entity it renders in place.
		self.children
			.iter()
			.flat_map(|children| children.iter())
			.filter_map(move |child| query.resolved_node(child).ok())
			.filter(|node| node.layout_style().display != Display::None)
	}

	/// Whether this node has any renderable child nodes.
	pub(super) fn has_child_nodes(&self, query: &CharcellQuery) -> bool {
		self.child_nodes(query).next().is_some()
	}
}

/// Follow a chain of [`RenderRef`] holders to the entity that renders in place.
///
/// A [`RenderRef`] holder is transparent: the charcell pipeline treats the
/// referenced entity as if it sat at the holder's position (see [`RenderRef`]),
/// so every traversal resolves through this before visiting a node.
pub(super) fn resolve_render_ref(
	refs: &Query<&RenderRef>,
	mut entity: Entity,
) -> Entity {
	while let Ok(render_ref) = refs.get(entity) {
		entity = **render_ref;
	}
	entity
}

/// Pre-order ([`RenderRef`]-resolved) traversal from `root`, inclusive.
///
/// Holders are skipped in favour of the entity they reference, matching
/// [`CharcellNodeData::child_nodes`] so the rect map keys line up.
pub(super) fn collect_pre_order(
	children: &Query<&Children>,
	refs: &Query<&RenderRef>,
	root: Entity,
) -> Vec<Entity> {
	let mut result = Vec::new();
	let mut stack = vec![resolve_render_ref(refs, root)];
	while let Some(entity) = stack.pop() {
		result.push(entity);
		if let Ok(children) = children.get(entity) {
			stack.extend(
				children.iter().rev().map(|child| resolve_render_ref(refs, child)),
			);
		}
	}
	result
}

/// Post-order ([`RenderRef`]-resolved) traversal from `root`, inclusive.
pub(super) fn collect_post_order(
	children: &Query<&Children>,
	refs: &Query<&RenderRef>,
	root: Entity,
) -> Vec<Entity> {
	let mut result = Vec::new();
	let mut stack = vec![(resolve_render_ref(refs, root), false)];
	while let Some((entity, visited)) = stack.pop() {
		if visited {
			result.push(entity);
		} else {
			stack.push((entity, true));
			if let Ok(children) = children.get(entity) {
				stack.extend(children.iter().rev().map(|child| {
					(resolve_render_ref(refs, child), false)
				}));
			}
		}
	}
	result
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
	refs: Query<'w, 's, &'static RenderRef>,
}

impl CharcellQuery<'_, '_> {
	/// Build a node, resolving [`RenderRef`] holders to the entity they render.
	pub(super) fn resolved_node(
		&self,
		entity: Entity,
	) -> Result<CharcellNodeData<'_>> {
		self.node(resolve_render_ref(&self.refs, entity))
	}

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
