use beet_bevy::prelude::*;
use beet_common::prelude::*;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;


/// Marker type for the root of the static scene.
#[derive(Debug, Clone, Default, Component)]
pub struct StaticSceneRoot;


pub(super) fn export_rsx_snippets(world: &mut World) -> bevy::prelude::Result {
	let snippets = world.run_system_once(collect_rsx_snippets)?;
	if snippets.is_empty() {
		return Ok(());
	}
	tracing::info!("Exporting {} rsx snippets", snippets.len());


	for (path, entities) in snippets.into_iter() {
		if let Some(config) = world.get_resource::<WorkspaceConfig>() {
			let scene = DynamicSceneBuilder::from_world(world)
				.extract_entities(entities.into_iter())
				.build();

			let scene = world.build_scene_with(scene);

			let out_path =
				config.rsx_snippets_dir().join(path).with_extension("ron");
			FsExt::write_if_diff(out_path, &scene)?;
		}
	}

	Ok(())
}

/// Collect all changed [`RsxSnippetRoot`]s, returning the output path
/// and all entities that are part of the snippet.
fn collect_rsx_snippets(
	query: Query<(Entity, &MacroIdx), Changed<RsxSnippetRoot>>,
	children: Query<&Children>,
) -> Vec<(WsPathBuf, Vec<Entity>)> {
	query
		.into_iter()
		.map(|(entity, idx)| {
			(
				idx.as_fs_path(),
				children.iter_descendants_inclusive(entity).collect(),
			)
		})
		.collect()
}


#[cfg(test)]
mod test {
	use std::path::Path;

	use crate::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin {
			skip_load_workspace: true,
			skip_write_to_fs: false,
			..default()
		});

		let test_site_index =
			WsPathBuf::new("crates/beet_router/src/test_site/pages/index.rs");
		let _entity = app
			.world_mut()
			.spawn(SourceFile::new(test_site_index.into_abs()))
			.id();

		app.update();

		let path = Path::new("target").join(test_site_index);
		let saved = ReadFile::to_string(path).unwrap();
		println!("Saved file: {}", saved);
	}
}
