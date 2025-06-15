use crate::prelude::*;
use beet_common::prelude::*;
use beet_net::prelude::*;
use beet_parse::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn parse_route_file_md(
	_: TempNonSendMarker,
	mut commands: Commands,
	group_codegen: Query<&CodegenFileSendit, With<FileGroup>>,
	parents: Query<&ChildOf>,
	mut query: Populated<(Entity, &mut RouteFile), Added<RouteFile>>,
) -> Result {
	for (entity, mut route_file) in query.iter_mut().filter(|(_, file)| {
		file.abs_path.extension().map_or(false, |ext| ext == "md")
	}) {
		let mut parent = commands.entity(entity);
		let file_str = ReadFile::to_string(&route_file.abs_path)?;

		let ws_path = route_file.abs_path.workspace_rel()?;
		let config = ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file_str);

		let Some(group_codegen) = parents
			.iter_ancestors(entity)
			.find_map(|e| group_codegen.get(e).ok())
		else {
			return Err(format!(
				"RouteFile has no CodegenFile for route file: {}",
				route_file.abs_path.display()
			)
			.into());
		};

		let default_path = WsPathBuf::default().into_abs();
		let group_codegen_dir =
			group_codegen.output.parent().unwrap_or(&default_path);


		let mut md_codegen_path = AbsPathBuf::new_unchecked(
			group_codegen_dir.join(&route_file.local_path),
		);
		md_codegen_path.set_extension("rs");


		parent.with_child(RouteFileMethod {
			config: if config.is_some() {
				RouteFileMethodConfig::Method
			} else {
				RouteFileMethodConfig::FileGroup
			},
			route_info: RouteInfo {
				path: RoutePath::from_file_path(&route_file.local_path)?,
				method: HttpMethod::Get,
			},
		});

		route_file.local_path = md_codegen_path.workspace_rel()?.take();
		// here the markdown will be generated in its own codegen,
		// it is seperate to the filegroup codegen tree
		commands.spawn((
			CombinatorTokens(rsx_str),
			SourceFile::new(ws_path.clone()),
			CombinatorRouteCodegen { config }.sendit(),
			CodegenFile::new(md_codegen_path).sendit(),
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

		let group = world.spawn(FileGroup::test_site_markdown()).id();
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
