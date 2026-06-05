//! Loading, watching and reloading the `beet.json` scene from the cwd.
//!
//! The `beet` binary is a scene runner. On startup [`load_beet_scene`] looks for
//! a `beet.json` in the cwd: absent, it renders [`SceneNotFound`] and exits;
//! present, it loads the scene (marking each root [`BeetSceneRoot`]) and installs
//! a [`BeetSceneWatcher`] that despawns and reloads the scene when the file
//! changes. The loaded scene supplies the actual CLI (a [`CliServer`] and its
//! routes), so the binary itself carries no commands.

mod scene_not_found;
pub use scene_not_found::*;
mod scene_watcher;
pub use scene_watcher::*;

use beet::prelude::*;

/// Loads, watches and reloads the `beet.json` scene from the cwd.
pub struct SceneManagementPlugin;

impl Plugin for SceneManagementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BeetSceneRoot>()
			.add_systems(Startup, load_beet_scene);
	}
}
