use crate::prelude::*;
use beet_common::prelude::*;
use beet_net::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn parse_route_file_md(
	mut commands: Commands,
	collection_codegen: Query<&CodegenFile, With<RouteFileCollection>>,
	parents: Query<&ChildOf>,
	mut query: Populated<
		(Entity, &SourceFile, &mut RouteFile),
		Changed<FileExprHash>,
	>,
) -> Result {
	for (entity, source_file, mut route_file) in
		query.iter_mut().filter(|(_, source, _)| {
			source.extension().map_or(false, |ext| ext == "md")
		}) {
		let mut parent = commands.entity(entity);
		parent.despawn_related::<Children>();

		let file_str = ReadFile::to_string(&source_file)?;

		let ws_path = source_file.into_ws_path()?;
		let meta = ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file_str);

		let Some(collection_codegen) = parents
			.iter_ancestors(entity)
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
		let mut route_codegen_path = route_file.collection_path.clone();
		route_codegen_path.set_extension("rs");

		let route_codegen_path_abs = AbsPathBuf::new_unchecked(
			collection_codegen_dir.join(&route_codegen_path),
		);

		parent.with_child(RouteFileMethod {
			meta: if meta.is_some() {
				RouteFileMethodMeta::File
			} else {
				RouteFileMethodMeta::Collection
			},
			route_info: RouteInfo {
				path: route_file.route_path.clone(),
				method: HttpMethod::Get,
			},
		});
		trace!("Parsed route file: {}", source_file.display());
		route_file.mod_path = route_codegen_path;
		// here the markdown will be generated in its own codegen
		parent.with_child((
			RsxSnippetRoot,
			MacroIdx::new(ws_path, LineCol::default()),
			CombinatorTokens::new(rsx_str),
			CombinatorRouteCodegen::new(meta),
			collection_codegen.clone_info(route_codegen_path_abs),
		));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_net::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let collection = world.spawn(RouteFileCollection::test_site_docs()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world.run_system_once(parse_route_file_md).unwrap().unwrap();
		let file = world.entity(collection).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let method = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();
		method
			.route_info
			.path
			.xpect()
			.to_be(RoutePath::new("/hello"));
	}
}
