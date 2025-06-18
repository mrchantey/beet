use crate::prelude::*;
use beet_common::prelude::*;
use beet_net::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn parse_route_file_md(
	_: TempNonSendMarker,
	mut commands: Commands,
	group_codegen: Query<&CodegenFileSendit, With<FileGroupSendit>>,
	parents: Query<&ChildOf>,
	mut query: Populated<(Entity, &mut RouteFile), Added<RouteFile>>,
) -> Result {
	for (entity, mut route_file) in query.iter_mut().filter(|(_, file)| {
		file.origin_path
			.extension()
			.map_or(false, |ext| ext == "md")
	}) {
		let mut parent = commands.entity(entity);
		let file_str = ReadFile::to_string(&route_file.origin_path)?;

		let ws_path = route_file.origin_path.into_ws_path()?;
		let config = ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file_str);

		let Some(group_codegen) = parents
			.iter_ancestors(entity)
			.find_map(|e| group_codegen.get(e).ok())
		else {
			return Err(format!(
				"RouteFile has no CodegenFile for route file: {}",
				route_file.origin_path.display()
			)
			.into());
		};

		let group_codegen_dir = group_codegen
			.output
			.parent()
			.unwrap_or_else(|| WsPathBuf::default().into_abs());

		// relative to the group codegen dir
		let mut route_codegen_path = route_file.group_path.clone();
		route_codegen_path.set_extension("rs");

		let route_codegen_path_abs = AbsPathBuf::new_unchecked(
			group_codegen_dir.join(&route_codegen_path),
		);

		parent.with_child(RouteFileMethod {
			meta: if config.is_some() {
				RouteFileMethodMeta::Method
			} else {
				RouteFileMethodMeta::FileGroup
			},
			route_info: RouteInfo {
				path: route_file.route_path.clone(),
				method: HttpMethod::Get,
			},
		});
		trace!("Parsed route file: {}", route_file.origin_path.display());
		route_file.mod_path = route_codegen_path;
		// here the markdown will be generated in its own codegen
		parent.with_child((
			CombinatorTokens(rsx_str),
			SourceFile::new(ws_path.clone()),
			CombinatorRouteCodegen { meta: config }.sendit(),
			group_codegen.clone_meta(route_codegen_path_abs).sendit(),
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

		let group = world.spawn(FileGroup::test_site_docs()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world.run_system_once(parse_route_file_md).unwrap().unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
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
