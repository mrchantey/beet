use crate::prelude::*;
use beet_core::prelude::*;

/// Activates [`Instance`]s: the documents they name loaded, held and built
/// under each use.
#[derive(Default)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<TemplatePlugin>()
			.install_type::<AsyncPlugin>()
			.init_resource::<PrefabCache>()
			.add_observer(crate::instance::activate_on_insert)
			.add_observer(crate::prefab_cache::forget_changed_prefabs);
	}
}
