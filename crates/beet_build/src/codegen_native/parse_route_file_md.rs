use crate::prelude::*;
use beet_parse::prelude::*;
use beet_router::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use syn::Signature;

pub fn parse_route_file_markdown(
	mut commands: Commands,
	group_codegen: Query<&CodegenFileSend, With<FileGroup>>,
	parents: Query<&ChildOf>,
	query: Populated<(Entity, &RouteFile), Added<RouteFile>>,
) -> Result<()> {
	for (
		entity,
		route_file @ RouteFile {
			abs_path,
			local_path,
			..
		},
	) in query.iter().filter(|(_, file)| {
		file.abs_path.extension().map_or(false, |ext| ext == "md")
	}) {
		let mod_ident = route_file.mod_ident();

		let mut parent = commands.entity(entity);
		let file_str = ReadFile::to_string(&route_file.abs_path)?;

		let ws_path = abs_path.workspace_rel()?;
		let frontmatter =
			ParseMarkdown::markdown_to_frontmatter_tokens(&file_str)?;
		let rsx_str = ParseMarkdown::markdown_to_rsx_str(&file_str);

		let Some(parent_codegen) = parents
			.iter_ancestors(entity)
			.find_map(|e| group_codegen.get(e).ok())
		else {
			return Err(format!(
				"RouteFile has no CodegenFile for route file: {}",
				abs_path.display()
			)
			.into());
		};

		let default_path = WorkspacePathBuf::default().into_abs();
		let group_codegen_dir =
			parent_codegen.output.parent().unwrap_or(&default_path);


		// here the markdown will be generated in its own codegen,
		// it is seperate to the filegroup codegen tree
		commands.spawn((
			CombinatorTokens(rsx_str),
			SourceFile::new(ws_path.clone()),
			CodegenFile::new(AbsPathBuf::new_unchecked(
				group_codegen_dir.join(local_path),
			))
			.sendit(),
		));

		let sig: Signature = syn::parse_quote! {
			fn get() -> impl Bundle
		};

		// parent.with_child(
		// 	RouteFileMethod {
		// 		mod_ident: mod_ident.clone(),
		// 		local_path,
		// 		abs_path,
		// 		mod_import: ModImport::Inline,
		// 		has_config: frontmatter,
		// 		method_path: sig,
		// 		route_info: RouteInfo {
		// 			path: RoutePath::from_file_path(&local_path)?,
		// 			method: HttpMethod::Get,
		// 		},
		// 	}
		// 	.sendit(),
		// );
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use std::ops::Deref;

	use crate::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use quote::ToTokens;
	use quote::quote;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let group = world.spawn(FileGroup::test_site_markdown()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world
			.run_system_once(parse_route_file_markdown)
			.unwrap()
			.unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let tokens = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();

	}
}
