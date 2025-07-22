use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::ReadFile;
use bevy::prelude::*;
use std::str::FromStr;
use syn::Visibility;



pub fn parse_route_file_rs(
	mut commands: Commands,
	source_files: Query<&SourceFile>,
	query: Populated<(Entity, &SourceFileRef, &RouteFile), Changed<RouteFile>>,
) -> Result {
	for (route_file_entity, source_file_ref, route_file) in
		query.iter().filter(|(_, _, route_file)| {
			route_file
				.source_file_collection_rel
				.extension()
				.map_or(false, |ext| ext == "rs")
		}) {
		let source_file = source_files.get(**source_file_ref)?;
		// discard any existing children, we could
		// possibly do a diff but these changes already result in recompile
		// so not super perf critical
		commands
			.entity(route_file_entity)
			.despawn_related::<Children>();

		let file_str = ReadFile::to_string(&source_file)?;

		// collect all public functions, including handlers and
		// possibly their meta functions
		let funcs = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(func) = item {
					match &func.vis {
						Visibility::Public(_) => {
							return Some((func.sig.ident.to_string(), func));
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		for (method, func) in funcs.iter().filter_map(|(ident, sig)| {
			HttpMethod::from_str(ident).ok().map(|method| (method, sig))
		}) {
			commands.spawn((
				ChildOf(route_file_entity),
				RouteFileMethodSyn::new(func.clone()),
				RouteFileMethod {
					route_info: RouteInfo::new(
						route_file.route_path.clone(),
						method,
					),
				},
			));
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::super::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		world.spawn(SourceFile::new(
			WsPathBuf::new(
				"crates/beet_router/src/test_site/pages/docs/index.rs",
			)
			.into_abs(),
		));


		let collection =
			world.spawn(RouteFileCollection::test_site_pages()).id();
		world
			.run_system_cached(update_route_files)
			.unwrap()
			.unwrap();
		world
			.run_system_cached(parse_route_file_rs)
			.unwrap()
			.unwrap();
		let file = world.entity(collection).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let route_method = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();
		// send_wrapper::SendWrapper::assert_send(&tokens);
		route_method
			.route_info
			.method
			.xpect()
			.to_be(HttpMethod::Get);
		route_method
			.route_info
			.path
			.xpect()
			.to_be(RoutePath::new("/docs"));
	}
}
