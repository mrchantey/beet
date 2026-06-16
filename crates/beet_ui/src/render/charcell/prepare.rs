//! Prepare phase: ensure all nodes in a buffer tree have the required layout
//! components before measure and layout systems run.
use super::*;
// explicit import shadows the bevy_ui `ScrollPosition` that leaks through
// `beet_core::prelude` when `bevy_default` is co-enabled.
use crate::input::ScrollPosition;
use crate::style::LayoutStyle;
use beet_core::prelude::*;

/// Insert [`IntrinsicSize`], [`LayoutRect`], and (on scroll containers)
/// [`ScrollPosition`] on every node in a buffer tree (rooted at a `B` component)
/// that is missing them.
///
/// Walks the tree via [`CharcellTree`], resolving [`Portal`] holders so
/// transcluded content is prepared too. Structural mutations are isolated to
/// this step so the measure, layout, and paint phases can run pure query access
/// without command buffering.
pub fn prepare_charcell_tree<B: Component>(
	mut commands: Commands,
	roots: Populated<Entity, With<B>>,
	tree: CharcellTree,
	has_intrinsic: Query<(), With<IntrinsicSize>>,
	has_layout: Query<(), With<LayoutRect>>,
	// a node becomes a scroll container when its resolved layout style scrolls an
	// axis; it then carries a persistent ScrollPosition.
	layout: Query<&LayoutStyle>,
	has_scroll: Query<(), With<ScrollPosition>>,
) {
	for root in roots.iter() {
		for entity in tree.pre_order(root) {
			if !has_intrinsic.contains(entity) {
				commands.entity(entity).insert(IntrinsicSize::default());
			}
			if !has_layout.contains(entity) {
				commands.entity(entity).insert(LayoutRect::default());
			}
			let scrolls = layout
				.get(entity)
				.is_ok_and(|style| style.overflow_x.is_scroll() || style.overflow_y.is_scroll());
			if scrolls && !has_scroll.contains(entity) {
				commands.entity(entity).insert(ScrollPosition::default());
			}
		}
	}
}
