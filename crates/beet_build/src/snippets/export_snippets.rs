use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn export_snippets(world: &mut World) -> bevy::prelude::Result {
	let snippets = world.run_system_cached(collect_rsx_snippets)?;
	if snippets.is_empty() {
		return Ok(());
	}
	tracing::info!("Exporting {} snippets", snippets.len());

	#[cfg(not(test))]
	{
		let scene = world.build_scene();
		let path = world
			.resource::<WorkspaceConfig>()
			.snippets_dir()
			.join("snippets.ron")
			.into_abs();
		tracing::info!("Writing one big snippet scene to {}", path.display());
		FsExt::write_if_diff(path, &scene)?;
	}

	// currently disabled until full roundtrip is stablized
	// doesnt work because rsx snippets are somehow relating to each other?
	// maybe templates..
	#[cfg(test)]
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

/// Collect all changed [`StaticRoot`]s, returning the output path
/// and all entities that are part of the snippet.
fn collect_rsx_snippets(
	config: Res<WorkspaceConfig>,
	query: Query<(Entity, &SnippetRoot), Changed<StaticRoot>>,
	children: Query<&Children>,
) -> Vec<(AbsPathBuf, Vec<Entity>)> {
	debug!("{} rsx snippets changed", query.iter().count());
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
	use beet_core::node::SnippetRoot;
	use beet_rsx::as_beet::*;
	// use beet_utils::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn rsx_snippets() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default())
			.insert_resource(BuildFlags::only(BuildFlag::ExportSnippets));


		let test_site_index =
			WsPathBuf::new("crates/beet_router/src/test_site/pages/index.rs");

		let snippet_path = WorkspaceConfig::default()
			.rsx_snippet_path(&SnippetRoot::new_file_line_col(
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
		saved.len().xpect_greater_than(1000);
	}
	#[test]
	#[ignore = "lang snippet exports is a wip"]
	fn lang_snippets() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default())
			.insert_resource(BuildFlags::only(BuildFlag::ExportSnippets));

		let path = WorkspaceConfig::default()
			.lang_snippet_path(&WsPathBuf::new(file!()), 0)
			.into_abs();

		// let _entity = app
		// 	.world_mut()
		// 	.spawn((HtmlDocument, rsx! {<style>div{color:blue;}</style>}))
		// 	.id();

		FsExt::remove(&path).ok();

		app.update();

		let saved = ReadFile::to_string(path).unwrap();
		// non-empty scene file
		saved.len().xpect_greater_than(200);
		#[cfg(feature = "css")]
		(&saved).xpect_contains("div[data-beet-style-id-0]");
		#[cfg(not(feature = "css"))]
		(&saved).xpect_contains("div{color:blue;}");
	}
}
