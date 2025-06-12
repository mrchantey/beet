use super::TemplateFile;
use super::error::Error;
use super::error::Result;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use beet_fs::prelude::*;
use beet_utils::prelude::*;


/// Config for the template creation stage of the build process
#[derive(Debug, Clone, Component, PartialEq, Serialize, Deserialize)]
pub struct BuildFileTemplates {
	/// Filter for files that should be parsed,
	/// excludes 'target' and 'node_modules' directories by default
	filter: GlobFilter,
	/// The root directory for files including templates
	root_dir: WorkspacePathBuf,
	/// The location for the generated template scene file
	scene_file: WorkspacePathBuf,
}

impl Default for BuildFileTemplates {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				// TODO move to beet.toml
				.with_include("*/crates/beet_design/src/**/*")
				.with_include("*/crates/beet_site/src/**/*")
				.with_include("*/crates/beet_router/src/test_site/**/*")
				.with_exclude("*/target/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			scene_file: WorkspacePathBuf::new("target/template_scene.ron"),
			#[cfg(test)]
			root_dir: WorkspacePathBuf::new("crates/beet_router/src/test_site"),
			#[cfg(not(test))]
			root_dir: WorkspacePathBuf::default(),
		}
	}
}

pub fn handle_changed_files(
	In(ev): In<WatchEventVec>,
	mut commands: Commands,
	builders: Query<&BuildFileTemplates>,
	query: Query<(Entity, &TemplateFile)>,
) -> bevy::prelude::Result {
	for ev in ev
		.mutated()
		.into_iter()
		// we only care about files that a builder will want to save
		.filter(|ev| {
			builders.iter().any(|config| config.filter.passes(&ev.path))
		}) {
		let workspace_path = ev.path.workspace_rel()?;

		// remove existing TemplateFile entities and their children
		for (entity, template_file) in query.iter() {
			if template_file.path() == &workspace_path {
				commands.entity(entity).despawn();
				tracing::debug!(
					"Removed TemplateFile entity for changed file: {}",
					workspace_path.display()
				);
			}
			//  else {
			// 	tracing::debug!(
			// 		"no match:\n{}\n{}",
			// 		workspace_path.display(),
			// 		template_file.path().display()
			// 	);
			// }
		}
		commands.spawn(TemplateFile::new(workspace_path));
	}
	Ok(())
}


/// Create a [`TemplateFile`] for each file specified in the [`BuildTemplatesConfig`].
pub(super) fn load_template_files(
	mut commands: Commands,
	query: Populated<&BuildFileTemplates, Added<BuildFileTemplates>>,
) -> bevy::prelude::Result {
	for config in query.iter() {
		config.get_files()?.into_iter().for_each(|path| {
			commands.spawn(TemplateFile::new(path));
		});
	}
	Ok(())
}

/// if any [`TemplateFile`] has changed, export the template scene
pub(super) fn export_template_scene(
	world: &mut World,
) -> bevy::prelude::Result {
	let mut entities = Vec::new();

	let changed_files = world
		.query_filtered::<&TemplateFile, Changed<TemplateFile>>()
		.iter(world)
		.collect::<Vec<_>>();

	if changed_files.is_empty() {
		// no changes, do nothing
		return Ok(());
	} else {
		tracing::info!(
			"Exporting {} template files to scene",
			changed_files.len()
		);
		tracing::debug!(
			"Changed template files: {}",
			changed_files
				.iter()
				.map(|f| f.path().to_string_lossy())
				.collect::<Vec<_>>()
				.join("\n")
		);
	}


	for (entity, config) in
		world.query::<(Entity, &BuildFileTemplates)>().iter(world)
	{
		let path = config.scene_file.clone();

		let scene = DynamicScene::from_world(world);

		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		// TODO only serialize TemplateRoot entities
		let scene = scene.serialize(&type_registry)?;

		FsExt::write(path.into_abs(), &scene)?;
		entities.push(entity);
	}

	Ok(())
}



impl BuildFileTemplates {
	pub fn get_files(&self) -> Result<Vec<WorkspacePathBuf>> {
		ReadDir::files_recursive(&self.root_dir.into_abs())
			.map_err(Error::File)?
			.into_iter()
			.filter(|path| self.filter.passes(path))
			.map(|path| {
				WorkspacePathBuf::new_cwd_rel(path).map_err(Error::File)
			})
			.collect::<Result<Vec<_>>>()
	}
}
