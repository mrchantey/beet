//! Small helpers for working with scenes, kept here so call sites stay terse.
use beet_core::prelude::*;
use bevy::app::TaskPoolPlugin;
use bevy::asset::AssetPlugin;
use bevy::scene::ScenePlugin;

/// A [`World`] wired with the minimal plugins required to spawn scenes:
/// [`TaskPoolPlugin`], [`AssetPlugin`] (for the [`AssetServer`] that
/// `spawn_scene` needs), and [`ScenePlugin`]. Insert any required resources on
/// the returned world before calling `spawn_scene`.
pub fn test_world() -> World {
	(TaskPoolPlugin::default(), AssetPlugin::default(), ScenePlugin)
		.into_world()
}
