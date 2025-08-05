use beet_core::prelude::WorldMutExt;
use beet_core::prelude::*;
use beet_utils::prelude::ReadDir;
use beet_utils::prelude::ReadFile;
use bevy::prelude::*;


/// Load all file snippets from the `snippets` directory on [`Startup`].
pub struct LoadSnippetsPlugin;

impl Plugin for LoadSnippetsPlugin {
	#[allow(unused)]
	fn build(&self, app: &mut App) {
		#[cfg(not(target_arch = "wasm32"))]
		app.init_plugin(crate::prelude::ApplyDirectivesPlugin)
			.add_systems(Startup, load_all_file_snippets);
	}
}


/// Load snippet scene if it exists.
// temp whole file until fine-grained loading is implemented
pub fn load_all_file_snippets(world: &mut World) -> Result {
	let config = world.resource::<WorkspaceConfig>();
	let file = config.snippets_dir().into_abs().join("snippets.ron");
	let file = ReadFile::to_string(file)?;
	world.load_scene(file)?;
	Ok(())
}
pub fn load_all_file_snippets_fine_grained(world: &mut World) -> Result {
	let config = world.resource::<WorkspaceConfig>();

	let files = ReadDir::files_recursive(config.snippets_dir().into_abs())?;
	let num_files = files.len();
	let start = std::time::Instant::now();
	// TODO fine-grained loading with watcher

	// TODO store this in a resource for hooking up with fine-grained loading
	let mut snippet_entity_map = Default::default();
	for file in files {
		let file = ReadFile::to_string(file)?;
		{
			world.load_scene_with(file, &mut snippet_entity_map)?;
		}
	}
	debug!(
		"Loaded {} file snippets in {}ms",
		num_files,
		start.elapsed().as_millis()
	);
	Ok(())
}
