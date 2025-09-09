use crate::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn parse_route_file_md(
	mut query: Populated<
		(Entity, &SourceFile, &mut RouteSourceFile),
		Added<SourceFile>,
	>,
	mut commands: Commands,
	collection_codegen: Query<&CodegenFile, With<RouteFileCollection>>,
	parents: Query<&ChildOf>,
) -> Result {
	for (entity, source_file, mut route_file) in
		query.iter_mut().filter(|(_, _, route_file)| {
			route_file
				.source_file_collection_rel
				.extension()
				.map_or(false, |ext| ext == "md" || ext == "mdx")
		}) {
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
		let mut route_codegen_path =
			route_file.source_file_collection_rel.clone();
		route_codegen_path.set_extension("rs");

		let route_codegen_path_abs = AbsPathBuf::new_unchecked(
			collection_codegen_dir.join(&route_codegen_path),
		);
		trace!("Parsed route file: {}", source_file.display());
		route_file.bypass_change_detection().mod_path = route_codegen_path;

		commands.spawn((
			ChildOf(entity),
			RouteFileMethod::new(RouteInfo {
				path: route_file.route_path.clone(),
				method: HttpMethod::Get,
			}),
		));
		// here the markdown will be generated in its own codegen
		commands.spawn((
			ChildOf(entity),
			CombinatorRouteCodegen,
			collection_codegen.clone_info(route_codegen_path_abs),
		));
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::super::*;
	use beet_net::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let collection =
			world.spawn(RouteFileCollection::test_site_docs()).id();
		world
			.run_system_cached(create_route_files)
			.unwrap()
			.unwrap();
		world
			.run_system_cached(parse_route_file_md)
			.unwrap()
			.unwrap();
		world
			.run_system_cached(parse_route_file_md)
			// bypass_change_detection
			.xpect_err();
		let file = world.entity(collection).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let method = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();
		method.route_info.path.xpect_eq(RoutePath::new("/"));
	}
}
