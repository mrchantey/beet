use beet_bevy::prelude::*;
use beet_common::prelude::*;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;


pub(super) fn export_lang_snippets(world: &mut World) -> bevy::prelude::Result {
	let snippets = world.run_system_once(collect_rsx_snippets)?;
	if snippets.is_empty() {
		return Ok(());
	}
	tracing::info!("Exporting {} rsx snippets", snippets.len());


	for (path, entities) in snippets.into_iter() {
		let scene = DynamicSceneBuilder::from_world(world)
			.extract_entities(entities.into_iter())
			.build();

		let scene = world.build_scene_with(scene);
		tracing::trace!("Writing rsx snippet to {}", path.display());
		FsExt::write_if_diff(path, &scene)?;
	}

	Ok(())
}

/// Collect all changed [`RsxSnippetRoot`]s, returning the output path
/// and all entities that are part of the snippet.
fn collect_rsx_snippets(
	config: Res<WorkspaceConfig>,
	query: Query<(Entity, &MacroIdx), Changed<RsxSnippetRoot>>,
	children: Query<&Children>,
) -> Vec<(AbsPathBuf, Vec<Entity>)> {
	query
		.into_iter()
		.map(|(entity, idx)| {
			(
				config.rsx_snippet_path(idx).into_abs(),
				children.iter_descendants_inclusive(entity).collect(),
			)
		})
		.collect()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::node::MacroIdx;
	use beet_router::as_beet::WorkspaceConfig;
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

		let snippet_path = WorkspaceConfig::default()
			.rsx_snippet_path(&MacroIdx::new_file_line_col(
				&test_site_index.to_string_lossy(),
				7,
				8,
			))
			.into_abs();

		let _entity = app
			.world_mut()
			.spawn(SourceFile::new(test_site_index.into_abs()))
			.id();

		FsExt::remove(&snippet_path).ok();

		app.update();

		let saved = ReadFile::to_string(snippet_path).unwrap();
		// non-empty scene file
		expect(saved.len()).to_be_greater_than(1000);
	}
}
