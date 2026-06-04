//! Prepare phase: ensure all nodes in a buffer tree have the required layout
//! components before measure and layout systems run.
use super::*;
use beet_core::prelude::*;

/// Insert [`IntrinsicSize`] and [`LayoutRect`] on every node in a buffer tree
/// (rooted at a `B` component) that is missing them.
///
/// Walks the tree via [`CharcellTree`], resolving [`RenderRef`] holders so
/// transcluded content is prepared too. Structural mutations are isolated to
/// this step so the measure, layout, and paint phases can run pure query access
/// without command buffering.
pub fn prepare_charcell_tree<B: Component>(
	mut commands: Commands,
	roots: Populated<Entity, With<B>>,
	tree: CharcellTree,
	has_intrinsic: Query<(), With<IntrinsicSize>>,
	has_layout: Query<(), With<LayoutRect>>,
) {
	for root in roots.iter() {
		for entity in tree.pre_order(root) {
			if !has_intrinsic.contains(entity) {
				commands.entity(entity).insert(IntrinsicSize::default());
			}
			if !has_layout.contains(entity) {
				commands.entity(entity).insert(LayoutRect::default());
			}
		}
	}
}
