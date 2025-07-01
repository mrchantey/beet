use crate::prelude::*;
use beet_bevy::prelude::WorldMutExt;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// Marker type for the root of the static scene.
#[derive(Debug, Clone, Default, Component)]
pub struct StaticSceneRoot;

/// Added alongside each [`SourceFile`] to distinguish them from a [`RouteFile`]
#[derive(Debug, Clone, Default, Component)]
pub struct SnippetFile;

/// if any [`SourceFile`] has been added, export the template scene
/// to the [`WorkspaceConfig::scene_file`].
#[allow(dead_code)]
pub(super) fn export_file_snippets(world: &mut World) -> bevy::prelude::Result {
	let changed_files = world
		.query_filtered_once::<&SourceFile, (With<SnippetFile>, Changed<SourceFile>)>(
		);

	if changed_files.is_empty() {
		// no changes, do nothing
		return Ok(());
	}
	// print the changed files
	tracing::info!("Exporting {} template files to scene", changed_files.len());

	let msg = if changed_files.len() > 5 {
		changed_files.len().to_string()
	} else {
		changed_files
			.iter()
			.map(|f| f.path().to_string_lossy())
			.collect::<Vec<_>>()
			.join("\n")
	};

	tracing::debug!("Exported {msg} template files to scene");


	// should really only be one of these
	if let Some(config) = world.get_resource::<WorkspaceConfig>() {
		let scene = world.build_scene();
		FsExt::write_if_diff(config.scene_file().into_abs(), &scene)?;
	}

	Ok(())
}
