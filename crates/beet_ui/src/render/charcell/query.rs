//! Core query types for the charcell render pipeline.
//!
//! Provides [`CharcellNodeData`] (a flat per-node style view without children),
//! and [`CharcellQuery`] (the shared system parameter).

use crate::prelude::*;
// explicit imports shadow the bevy_ui types of the same name that leak through
// `beet_core::prelude` when `bevy_default` is co-enabled.
use crate::input::ScrollPosition;
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
	element: Option<&'a Element>,
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
	transition: Option<&'a VisualTransition>,
	kitty: Option<&'a KittyImage>,
}

impl CharcellNodeData<'_> {
	/// The node's displayed [`Value`]: a text leaf's content, or a form
	/// control's bound value. An element's own `Value` is otherwise binding
	/// state (eg a `bx:click` field mirror), never painted as text.
	pub fn value(&self) -> Option<&Value> {
		self.element
			.is_none_or(|element| is_value_element(element.tag()))
			.then_some(self.value)
			.flatten()
	}

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

	/// The visual style to display: the in-flight [`VisualTransition`] value
	/// when the element is transitioning, else the resolved target style,
	/// defaulting to [`VISUAL_STYLE_DEFAULT`].
	pub fn visual_style(&self) -> &VisualStyle {
		self.transition
			.map(|transition| &transition.current)
			.unwrap_or_else(|| self.visual.unwrap_or(&VISUAL_STYLE_DEFAULT))
	}

	/// OSC-8 hyperlink target, if this element is an `<a>`/`<img>`.
	pub fn hyperlink(&self) -> Option<&str> {
		self.hyperlink.map(|link| link.0.as_str())
	}

	/// Generated leading content (bullet, quote bar, rule, alt text), if any.
	pub fn marker(&self) -> Option<&str> {
		self.marker.map(|marker| marker.0.as_str())
	}

	/// The kitty-graphics raster backing this `<img>`, if attached.
	pub fn kitty_image(&self) -> Option<&KittyImage> { self.kitty }
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
		// [`Portal`] holder is resolved to the entity it renders in place; a
		// tag-less grouping wrapper (children, no [`Element`]) is spliced out, its
		// children hoisted into this flow so they lay out as direct siblings.
		query
			.flow_child_entities(self.children)
			.into_iter()
			.filter_map(move |child| query.unresolved_node(child).ok())
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

/// Follow a chain of [`Portal`] holders to the entity that renders in place.
///
/// A [`Portal`] holder is transparent: the charcell pipeline treats the
/// referenced entity as if it sat at the holder's position (see [`Portal`]),
/// so every traversal resolves through this before visiting a node.
fn resolve_render_ref(refs: &Query<&Portal>, mut entity: Entity) -> Entity {
	// follow holders to their target; the relationship always names one, so an
	// unresolved slot points at a placeholder that renders empty in place.
	while let Ok(render_ref) = refs.get(entity) {
		entity = render_ref.target();
	}
	entity
}

/// Detects a *transparent grouping wrapper*: a node that groups children without
/// introducing a box of its own, so every traversal hoists its children into the
/// parent's flow rather than treating it as a nested box. This is the shape a
/// collected `Vec`/iterator child position lowers to (eg `{cells.collect()}`); a
/// `<div>`, an anonymous block, or a hand-styled box is *not* one.
///
/// Mirrors the HTML walker's tag-less transparency (it emits a tag only for an
/// [`Element`] yet recurses into children regardless): the primary signal is "no
/// [`Element`]". Charcell adds one box-establishing carve-out, since a tag-less
/// node *can* carry a hand-attached load-bearing style in tests and ad-hoc trees:
/// a non-default [`BoxStyle`] (border/padding/size) or a [`VisualStyle`]
/// background needs the node's own rect to paint, so such a node stays a box.
///
/// These two are the reliable signals after `resolve_styles`, which gives every
/// node a resolved style: it leaves a non-element's [`BoxStyle`] at the default
/// (the box model is element-only) and never inherits a `background` (it is a
/// non-inherited property), so a hoisted wrapper has neither while an authored box
/// has one. The resolved [`LayoutStyle`] is *not* a usable signal — a tag-less
/// node resolves its `display` from its nearest ancestor element, so a fragment
/// under a `display: grid` parent picks up `Grid`; its display is simply never
/// read, as it is spliced before layout.
#[derive(SystemParam)]
pub(crate) struct WrapperQuery<'w, 's> {
	wrappers: Query<
		'w,
		's,
		(
			&'static Children,
			Option<&'static BoxStyle>,
			Option<&'static VisualStyle>,
		),
		Without<Element>,
	>,
}

impl WrapperQuery<'_, '_> {
	/// The children to hoist when `entity` is a transparent grouping wrapper, else
	/// `None` (it is a real node and lays out as itself).
	fn splice_children(&self, entity: Entity) -> Option<&Children> {
		let (children, box_style, visual) = self.wrappers.get(entity).ok()?;
		// transparent unless the node authored a box of its own: a non-default box
		// model or a background fill both need the node's own painted rect.
		(box_style.is_none_or(|style| *style == BoxStyle::default())
			&& visual.is_none_or(|style| style.background.is_none()))
		.then_some(children)
	}
}

/// [`Portal`]-aware traversal of a charcell buffer tree.
///
/// Every phase (prepare, measure, layout, paint) walks the tree through this, so
/// a [`Portal`] holder is transparently replaced by the entity it renders in
/// a single place rather than each system threading [`Children`] + [`Portal`]
/// queries and re-deriving the resolution. Its orderings match
/// [`CharcellNodeData::child_nodes`], so the per-entity rect map keys line up.
#[derive(SystemParam)]
pub(crate) struct CharcellTree<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	refs: Query<'w, 's, &'static Portal>,
	// transparent grouping wrappers spliced out so every traversal agrees with
	// [`CharcellNodeData::child_nodes`] (see [`WrapperQuery`]).
	wrappers: WrapperQuery<'w, 's>,
}

impl CharcellTree<'_, '_> {
	/// Follow [`Portal`] holders to the entity that renders in place.
	pub fn resolve(&self, entity: Entity) -> Entity {
		resolve_render_ref(&self.refs, entity)
	}

	/// Resolved children of `entity`: holders replaced by their referents and
	/// tag-less grouping wrappers spliced out so their children are hoisted.
	fn children(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
		let mut out = Vec::new();
		for child in self
			.children
			.get(entity)
			.into_iter()
			.flat_map(|children| children.iter())
		{
			self.push_flow_entity(child, &mut out);
		}
		out.into_iter()
	}

	/// Resolve `child` into the flow, recursing through transparent wrappers so
	/// their children take their place.
	fn push_flow_entity(&self, child: Entity, out: &mut Vec<Entity>) {
		let child = self.resolve(child);
		match self.wrappers.splice_children(child) {
			Some(grandchildren) => {
				for grandchild in grandchildren.iter() {
					self.push_flow_entity(grandchild, out);
				}
			}
			None => out.push(child),
		}
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
			Option<&'static Element>,
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
			Option<&'static VisualTransition>,
			Option<&'static KittyImage>,
		),
	>,
	refs: Query<'w, 's, &'static Portal>,
	// transparent grouping wrappers (see [`WrapperQuery`]); their children are
	// hoisted into the parent's flow. Such a wrapper carries no box, so the render
	// systems never assign it an `IntrinsicSize`/`LayoutRect` and it is absent from
	// `nodes`: `WrapperQuery` reads its [`Children`] directly so the hoist still
	// finds them.
	wrappers: WrapperQuery<'w, 's>,
}

impl CharcellQuery<'_, '_> {
	/// The flow child entities behind a [`Children`]: each resolved through
	/// [`Portal`] holders, with transparent grouping wrappers spliced out so
	/// their own children take their place (depth-first, order preserved). Keeps
	/// [`CharcellNodeData::child_nodes`] aligned with the [`CharcellTree`]
	/// traversal that paint reads.
	fn flow_child_entities(&self, children: Option<&Children>) -> Vec<Entity> {
		let mut out = Vec::new();
		for child in children.iter().flat_map(|children| children.iter()) {
			self.push_flow_entity(child, &mut out);
		}
		out
	}

	/// Resolve a child into the flow: push its [`Portal`]-resolved id, or, when
	/// it is a transparent wrapper, recurse so its children take its place.
	fn push_flow_entity(&self, entity: Entity, out: &mut Vec<Entity>) {
		let entity = resolve_render_ref(&self.refs, entity);
		// the wrapper is checked independently of `nodes`: it has no box, so it is
		// never prepared into `nodes`, but its children must still be hoisted.
		match self.wrappers.splice_children(entity) {
			Some(children) => {
				for child in children.iter() {
					self.push_flow_entity(child, out);
				}
			}
			None => out.push(entity),
		}
	}

	/// Build a node for an entity that is already [`Portal`]-resolved,
	/// without following holders again.
	pub(super) fn unresolved_node(
		&self,
		entity: Entity,
	) -> Result<CharcellNodeData<'_>> {
		let (
			intrinsic_size,
			layout_rect,
			element,
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
			transition,
			kitty,
		) = self.nodes.get(entity)?;
		Ok(CharcellNodeData {
			intrinsic_size,
			layout_rect,
			entity,
			element,
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
			transition,
			kitty,
		})
	}
}
