//! Core query types for the charcell render pipeline.
//!
//! Provides [`CharcellNodeData`] (a flat per-node style view without children),
//! and [`CharcellQuery`] (the shared system parameter).

use crate::prelude::*;
use crate::style::Display;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::IRect;
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
	scroll: Option<&'a ScrollPosition>,
	position: Option<&'a PositionStyle>,
	scrollbar: Option<&'a ScrollbarStyle>,
}

impl CharcellNodeData<'_> {
	pub fn value(&self) -> Option<&Value> { self.value }

	pub fn intrinsic_size(&self) -> UVec2 { self.intrinsic_size.0 }
	pub fn layout_rect(&self) -> IRect { self.layout_rect.0 }

	/// This node's own scroll offset (zero when not a scroll container), the
	/// distance the paint pass translates its descendants by.
	pub fn scroll_offset(&self) -> bevy::math::IVec2 {
		self.scroll.map(|scroll| scroll.offset).unwrap_or_default()
	}

	/// Whether this node is a scroll container (has a [`ScrollPosition`]).
	pub fn is_scroll_container(&self) -> bool { self.scroll.is_some() }

	/// Resolved positioning, defaulting to static with no insets.
	pub fn position_style(&self) -> PositionStyle {
		self.position.copied().unwrap_or_default()
	}

	/// Resolved scrollbar styling, defaulting to the renderer defaults.
	pub fn scrollbar_style(&self) -> ScrollbarStyle {
		self.scrollbar.copied().unwrap_or_default()
	}

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

	/// In-flow child nodes: [`child_nodes`](Self::child_nodes) minus out-of-flow
	/// (absolute/fixed) children, which the flow layout must ignore so siblings
	/// lay out as if they are absent. The out-of-flow children are placed against
	/// their containing block in the positioning pass.
	pub(super) fn flow_child_nodes<'a>(
		&'a self,
		query: &'a CharcellQuery,
	) -> impl 'a + Iterator<Item = CharcellNodeData<'a>> {
		self.child_nodes(query)
			.filter(|node| !node.position_style().position.is_out_of_flow())
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
fn resolve_render_ref(refs: &Query<&RenderRef>, mut entity: Entity) -> Entity {
	while let Ok(render_ref) = refs.get(entity) {
		entity = **render_ref;
	}
	entity
}

/// [`RenderRef`]-aware traversal of a charcell buffer tree.
///
/// Every phase (prepare, measure, layout, paint) walks the tree through this, so
/// a [`RenderRef`] holder is transparently replaced by the entity it renders in
/// a single place rather than each system threading [`Children`] + [`RenderRef`]
/// queries and re-deriving the resolution. Its orderings match
/// [`CharcellNodeData::child_nodes`], so the per-entity rect map keys line up.
#[derive(SystemParam)]
pub(crate) struct CharcellTree<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	refs: Query<'w, 's, &'static RenderRef>,
}

impl CharcellTree<'_, '_> {
	/// Follow [`RenderRef`] holders to the entity that renders in place.
	pub fn resolve(&self, entity: Entity) -> Entity {
		resolve_render_ref(&self.refs, entity)
	}

	/// Resolved children of `entity`, holders replaced by their referents.
	fn children(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
		self.children
			.get(entity)
			.into_iter()
			.flat_map(|children| children.iter())
			.map(|child| self.resolve(child))
	}

	/// Resolved direct children of `entity`, holders replaced by their referents.
	///
	/// Same ordering as [`CharcellNodeData::child_nodes`], so the per-entity rect
	/// and clip maps key consistently with paint.
	pub fn children_of(&self, entity: Entity) -> Vec<Entity> {
		self.children(entity).collect()
	}

	/// Pre-order traversal from `root`, inclusive.
	pub fn pre_order(&self, root: Entity) -> Vec<Entity> {
		let mut result = Vec::new();
		let mut stack = vec![self.resolve(root)];
		while let Some(entity) = stack.pop() {
			result.push(entity);
			stack.extend(
				self.children(entity).collect::<Vec<_>>().into_iter().rev(),
			);
		}
		result
	}

	/// Post-order traversal from `root`, inclusive.
	pub fn post_order(&self, root: Entity) -> Vec<Entity> {
		let mut result = Vec::new();
		let mut stack = vec![(self.resolve(root), false)];
		while let Some((entity, visited)) = stack.pop() {
			if visited {
				result.push(entity);
			} else {
				stack.push((entity, true));
				stack.extend(
					self.children(entity)
						.collect::<Vec<_>>()
						.into_iter()
						.rev()
						.map(|child| (child, false)),
				);
			}
		}
		result
	}

	/// Resolved descendants of `entity`, excluding `entity` itself.
	pub fn descendants(&self, entity: Entity) -> impl Iterator<Item = Entity> {
		self.pre_order(entity).into_iter().skip(1)
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
			Option<&'static ScrollPosition>,
			Option<&'static PositionStyle>,
			Option<&'static ScrollbarStyle>,
		),
	>,
	refs: Query<'w, 's, &'static RenderRef>,
}

impl CharcellQuery<'_, '_> {
	/// Build a node, resolving [`RenderRef`] holders to the entity they render.
	///
	/// Use this when starting from a raw [`Children`] entity. Once an entity has
	/// already been resolved (eg by [`CharcellTree`] traversal), call
	/// [`Self::unresolved_node`] to avoid a redundant resolution.
	pub(super) fn resolved_node(
		&self,
		entity: Entity,
	) -> Result<CharcellNodeData<'_>> {
		self.unresolved_node(resolve_render_ref(&self.refs, entity))
	}

	/// Build a node for an entity that is already [`RenderRef`]-resolved,
	/// without following holders again.
	pub(super) fn unresolved_node(
		&self,
		entity: Entity,
	) -> Result<CharcellNodeData<'_>> {
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
			scroll,
			position,
			scrollbar,
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
			scroll,
			position,
			scrollbar,
		})
	}
}
