//! Snippet export system for saving parsed RSX snippets to disk.
//!
//! This module handles exporting [`StaticRoot`] entities and their descendants
//! to RON scene files that can be loaded at runtime for client-side hydration.

use beet_core::prelude::*;
use beet_dom::prelude::*;


/// Exports all changed snippets to disk.
///
/// This function determines the export strategy and writes snippet scene files
/// to the configured snippets directory.
pub(crate) fn export_snippets(world: &mut World) -> Result {
	// #[cfg(not(test))]
	// export_all_snippets(world)?;
	// #[cfg(test)]
	export_snippets_incrementally(world)?;
	Ok(())
}

/// Exports all snippets to a single scene file.
#[allow(unused)]
fn export_all_snippets(world: &mut World) -> Result {
	// temp hack: just put them all in one big file
	let scene = world.build_scene();
	let path = world
		.resource::<WorkspaceConfig>()
		.snippets_dir()
		.join("snippets.ron")
		.into_abs();
	tracing::info!("Writing one big snippet scene to {}", path.display());
	fs_ext::write_if_diff(path, &scene)?;
	Ok(())
}

/// Exports snippets incrementally, one file per changed snippet root.
fn export_snippets_incrementally(world: &mut World) -> Result {
	// currently disabled until full roundtrip is stablized
	// doesnt work because rsx snippets are somehow relating to each other?
	// maybe templates..
	let file_snippets = world
		.run_system_cached(collect_changed_snippet_files)
		.unwrap_or_default();
	tracing::info!("Exporting snippets for {} files", file_snippets.len());

	for file_snippets in file_snippets.into_iter() {
		// temporarily remove parent to avoid 'entity not found'
		let parent = world.entity_mut(file_snippets.root).take::<ChildOf>();

		let scene = DynamicSceneBuilder::from_world(world)
			// .deny_component::<CodegenFile>()
			// .deny_component::<MetaType>()
			// .deny_component::<RouteFileCollection>()
			// .deny_component::<ModifyRoutePath>()
			.extract_entities(file_snippets.entities.clone().into_iter())
			.build();

		let scene = world.build_scene_with(scene);
		tracing::trace!(
			"Writing rsx snippet to {}",
			file_snippets.path.display()
		);
		fs_ext::write_if_diff(&file_snippets.path, &scene)?;
		if let Some(parent) = parent {
			world.entity_mut(file_snippets.root).insert(parent);
		}
	}
	Ok(())
}

/// Information about a file's snippets to be exported.
struct FileSnippets {
	/// The root entity of the snippet.
	root: Entity,
	/// The output path for the exported scene file.
	path: AbsPathBuf,
	/// All entities that are part of this snippet (root + descendants).
	entities: Vec<Entity>,
}

/// Collects all changed [`StaticRoot`] entities and their export information.
///
/// Returns the output path and all entities that are part of each snippet.
#[cfg_attr(not(test), allow(unused))]
fn collect_changed_snippet_files(
	config: Res<WorkspaceConfig>,
	query: Populated<(Entity, &SnippetRoot), Changed<StaticRoot>>,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
) -> Vec<FileSnippets> {
	debug!("{} rsx snippets changed", query.iter().count());
	query
		.into_iter()
		.map(|(entity, idx)| FileSnippets {
			root: entity,
			path: config
				.rsx_snippet_path(&idx.file, idx.start.line)
				.into_abs(),
			entities: children
				.iter_descendants_inclusive(entity)
				.flat_map(|entity| {
					if let Ok(attrs) = attributes.get(entity) {
						let mut entities = vec![entity];
						entities.extend(&**attrs);
						entities
					} else {
						vec![entity]
					}
				})
				.collect(),
		})
		.collect()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn rsx_snippets() {
		let mut world = BuildPlugin::world();


		let test_site_index = WsPathBuf::new("tests/test_site/pages/index.rs");

		let snippet_path = WorkspaceConfig::default()
			.rsx_snippet_path(&test_site_index, 7)
			.into_abs();

		let _entity = world
			.spawn(SourceFile::new(test_site_index.into_abs()))
			.id();

		fs_ext::remove(&snippet_path).ok();

		world.run_schedule(ParseSourceFiles);

		let saved = fs_ext::read_to_string(snippet_path).unwrap();
		// non-empty scene file
		saved.len().xpect_greater_than(500);
	}
	#[test]
	#[ignore = "lang snippet exports is a wip"]
	fn lang_snippets() {
		let mut world = BuildPlugin::world();

		let path = WorkspaceConfig::default()
			.lang_snippet_path(&WsPathBuf::new(file!()), 0)
			.into_abs();

		// let _entity = app
		// 	.world_mut()
		// 	.spawn((HtmlDocument, rsx! {<style>div{color:blue;}</style>}))
		// 	.id();

		fs_ext::remove(&path).ok();

		world.run_schedule(ParseSourceFiles);

		let saved = fs_ext::read_to_string(path).unwrap();
		// non-empty scene file
		saved.len().xpect_greater_than(200);
		#[cfg(feature = "css")]
		(&saved).xpect_contains("div[data-beet-style-id-0]");
		#[cfg(not(feature = "css"))]
		(&saved).xpect_contains("div{color:blue;}");
	}
}
