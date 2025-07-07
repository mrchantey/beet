use crate::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn parse_route_file_md(
	mut query: Populated<
		(Entity, &SourceFileRef, &mut RouteFile),
		Changed<RouteFile>,
	>,
	mut commands: Commands,
	source_files: Query<&SourceFile>,
	collection_codegen: Query<&CodegenFile, With<RouteFileCollection>>,
	parents: Query<&ChildOf>,
) -> Result {
	for (route_file_entity, source_file_ref, mut route_file) in
		query.iter_mut().filter(|(_, _, route_file)| {
			route_file
				.source_file_collection_rel
				.extension()
				.map_or(false, |ext| ext == "md" || ext == "mdx")
		}) {
		let source_file = source_files.get(**source_file_ref)?;
		commands
			.entity(route_file_entity)
			.despawn_related::<Children>();

		// discard any existing children, we could
		// possibly do a diff but these changes already result in recompile
		// so not super perf critical

		// loading the file a second time is not ideal, we should probably
		// cache the meta from the first parse
		let file_str = ReadFile::to_string(&source_file)?;
		let meta = ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;

		let Some(collection_codegen) = parents
			.iter_ancestors(route_file_entity)
			.find_map(|e| collection_codegen.get(e).ok())
		else {
			return Err(format!(
				"RouteFile has no CodegenFile for route file: {}",
				source_file.display()
			)
			.into());
		};

		let collection_codegen_dir = collection_codegen
			.output
			.parent()
			.unwrap_or_else(|| WsPathBuf::default().into_abs());

		// relative to the collection codegen dir
		let mut route_codegen_path =
			route_file.source_file_collection_rel.clone();
		route_codegen_path.set_extension("rs");

		let route_codegen_path_abs = AbsPathBuf::new_unchecked(
			collection_codegen_dir.join(&route_codegen_path),
		);
		trace!("Parsed route file: {}", source_file.display());
		route_file.bypass_change_detection().mod_path = route_codegen_path;

		commands.spawn((ChildOf(route_file_entity), RouteFileMethod {
			meta: if meta.is_some() {
				RouteFileMethodMeta::File
			} else {
				RouteFileMethodMeta::Collection
			},
			route_info: RouteInfo {
				path: route_file.route_path.clone(),
				method: HttpMethod::Get,
			},
		}));
		// here the markdown will be generated in its own codegen
		commands.spawn((
			ChildOf(route_file_entity),
			CombinatorRouteCodegen::new(meta),
			SourceFileRef(**source_file_ref),
			collection_codegen.clone_info(route_codegen_path_abs),
		));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::super::*;
	use crate::prelude::*;
	use beet_net::prelude::*;
	use beet_utils::prelude::WsPathBuf;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		world.spawn(SourceFile::new(
			WsPathBuf::new(
				"crates/beet_router/src/test_site/test_docs/index.mdx",
			)
			.into_abs(),
		));


		let collection =
			world.spawn(RouteFileCollection::test_site_docs()).id();
		world
			.run_system_cached(update_route_files)
			.unwrap()
			.unwrap();
		world
			.run_system_cached(parse_route_file_md)
			.unwrap()
			.unwrap();
		world
			.run_system_cached(parse_route_file_md)
			.xpect()
			// bypass_change_detection
			.to_be_err();
		let file = world.entity(collection).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let method = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();
		method.route_info.path.xpect().to_be(RoutePath::new("/"));
	}
}
