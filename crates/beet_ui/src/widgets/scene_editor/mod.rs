//! The scene editor: the widgets that edit the scene document a page is.
//!
//! **Edit the document, never the world.** Every widget here reads and writes
//! the [`SceneDocument`] of the scene it lives in, and the world follows per
//! component; nothing reaches into a live entity. The editor is therefore the
//! same generated machinery as any other document editor: the
//! [`EntityInspector`] is a [`DynamicForm`](super::DynamicForm) over the
//! registered [`ValueSchema::scene_entity`] schema at the selected entity's
//! path, the [`SceneTree`] a generation over the document's entities, and a
//! reparent is the `ChildOf` entry's entity picker.
//!
//! The editor lives *inside* the scene it edits: [`ToggleSceneEditor`] is
//! authored as a tag in the scene, stays in the scene document (so removing
//! your editor is an ordinary edit) and spawns its widgets as [`Derived`]
//! furniture the fork never dumps. Selection is a [`SceneSelection`] on the
//! scene document's host, per document rather than global.
mod entity_inspector;
mod scene_editor;
mod scene_tree;
mod style;
/// Harness for the scene editor tests.
#[cfg(test)]
mod test_ext;

pub use entity_inspector::*;
pub use scene_editor::*;
pub use scene_tree::*;
use style::*;

use beet_core::prelude::*;

/// Installs the scene editor into the widget set: its tag registered so a
/// fork keeps it, its widgets by short type path, the systems binding an
/// editor to its scene and following the selection, and its rules.
pub(super) fn scene_editor_plugin(app: &mut App) {
	app.register_type::<ToggleSceneEditor>()
		.register_template::<SceneEditor>()
		.register_template::<SceneTree>()
		.register_template::<EntityInspector>()
		.add_systems(
			Update,
			(
				link_scene_editors,
				clear_stale_selections,
				rebuild_inspectors,
				sync_tree_selection,
			)
				.chain(),
		);
	app.world_mut()
		.get_resource_or_init::<crate::prelude::RuleSet>()
		.extend_rules(scene_editor_rules());
}
