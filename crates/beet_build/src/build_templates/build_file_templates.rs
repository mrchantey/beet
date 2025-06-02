use super::TemplateFile;
use super::error::Error;
use super::error::Result;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;


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


pub(super) fn export_template_scene(
	world: &mut World,
) -> bevy::prelude::Result {
	let mut entities = Vec::new();

	for (entity, config) in
		world.query::<(Entity, &BuildFileTemplates)>().iter(world)
	{
		let path = config.scene_file.clone();

		let scene = DynamicScene::from_world(world);

		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		let scene = scene.serialize(&type_registry)?;

		FsExt::write(path.into_abs_unchecked(), &scene)?;
		entities.push(entity);
	}
	for entity in entities {
		world.entity_mut(entity).despawn();
	}


	Ok(())
}



impl BuildFileTemplates {
	pub fn get_files(&self) -> Result<Vec<WorkspacePathBuf>> {
		ReadDir::files_recursive(
			&self.root_dir.into_abs().map_err(Error::File)?,
		)
		.map_err(Error::File)?
		.into_iter()
		.filter(|path| self.filter.passes(path))
		.map(|path| {
			WorkspacePathBuf::new_from_cwd_rel(path).map_err(Error::File)
		})
		.collect::<Result<Vec<_>>>()
	}
}
