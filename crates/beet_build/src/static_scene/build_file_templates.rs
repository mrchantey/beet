use super::TemplateFile;
use beet_bevy::prelude::When;
use beet_bevy::prelude::WorldMutExt;
use beet_fs::prelude::*;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


/// Create a [`TemplateFile`] for each file specified in the [`StaticSceneConfig`].
/// This will run once for the initial load, afterwards [`handle_changed_files`]
/// will incrementally load changed files.
pub(super) fn load_all_template_files(
	mut commands: Commands,
	config: When<Res<StaticSceneConfig>>,
) -> bevy::prelude::Result {
	config.get_files()?.into_iter().for_each(|path| {
		commands.spawn(TemplateFile::new(path));
	});
	Ok(())
}

/// When a file is changed
pub(super) fn load_changed_template_files(
	mut events: EventReader<WatchEvent>,
	mut commands: Commands,
	config: When<Res<StaticSceneConfig>>,
	query: Query<(Entity, &TemplateFile)>,
) -> bevy::prelude::Result {
	for ev in events
		.read()
		// we only care about files that a builder will want to save
		.filter(|ev| config.passes(&ev.path))
	{
		let ws_path = ev.path.into_ws_path()?;

		// recursively remove existing TemplateFile entities
		for (entity, _) in query
			.iter()
			.filter(|(_, template_file)| template_file.path() == &ws_path)
		{
			commands.entity(entity).despawn();
			tracing::debug!(
				"Removed TemplateFile entity for changed file: {}",
				ws_path.display()
			);
		}
		commands.spawn(TemplateFile::new(ws_path));
	}
	Ok(())
}


/// if any [`TemplateFile`] has changed, export the template scene.
/// This does nothing without a [`StaticSceneConfig`] resource
#[allow(dead_code)]
pub(super) fn export_template_scene(
	world: &mut World,
) -> bevy::prelude::Result {
	let changed_files =
		world.query_filtered_once::<&TemplateFile, Changed<TemplateFile>>();

	if changed_files.is_empty() {
		// no changes, do nothing
		return Ok(());
	} else {
		// print the changed files
		tracing::info!(
			"Exporting {} template files to scene",
			changed_files.len()
		);

		let msg = if changed_files.len() > 5 {
			changed_files.len().to_string()
		} else {
			changed_files
				.iter()
				.map(|f| f.path().to_string_lossy())
				.collect::<Vec<_>>()
				.join("\n")
		};

		tracing::debug!("Changed template files: {msg}",);
	}

	// should really only be one of these
	if let Some(config) = world.get_resource::<StaticSceneConfig>() {
		let scene = world.build_scene();
		FsExt::write_if_diff(config.scene_file().into_abs(), &scene)?;
	}

	Ok(())
}
