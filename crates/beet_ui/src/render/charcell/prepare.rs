//! Prepare phase: ensure all nodes in a [`DoubleBuffer`] tree have the
//! required layout components before measure and layout systems run.
use beet_core::prelude::*;

use super::DoubleBuffer;
use super::IntrinsicSize;
use super::LayoutRect;

/// Insert [`IntrinsicSize`] and [`LayoutRect`] on every node in a
/// [`DoubleBuffer`] tree that is missing them.
///
/// Structural mutations are isolated to this step so the measure, layout,
/// and paint phases can run pure query access without command buffering.
pub fn prepare_charcell_tree(
	mut commands: Commands,
	roots: Query<Entity, With<DoubleBuffer>>,
	children: Query<&Children>,
	has_intrinsic: Query<(), With<IntrinsicSize>>,
	has_layout: Query<(), With<LayoutRect>>,
) {
	for root in roots.iter() {
		for entity in children.iter_descendants_inclusive(root) {
			if !has_intrinsic.contains(entity) {
				commands.entity(entity).insert(IntrinsicSize::default());
			}
			if !has_layout.contains(entity) {
				commands.entity(entity).insert(LayoutRect::default());
			}
		}
	}
}
