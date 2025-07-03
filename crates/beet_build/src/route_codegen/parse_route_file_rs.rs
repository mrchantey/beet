use crate::prelude::*;
use beet_net::prelude::*;
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
		commands.entity(route_file_entity).despawn_related::<Children>();

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

		for (ident, method, func) in funcs.iter().filter_map(|(ident, sig)| {
			HttpMethod::from_str(ident)
				.ok()
				.map(|method| (ident, method, sig))
		}) {
			let meta_ident = format!("meta_{}", ident);
			let meta = funcs
				.iter()
				.find_map(|(ident, _)| match ident.as_str() {
					"meta" => Some(RouteFileMethodMeta::File),
					ident if ident == &meta_ident => {
						Some(RouteFileMethodMeta::Method)
					}
					_ => None,
				})
				.unwrap_or_default();

			commands.spawn((
				ChildOf(route_file_entity),
				RouteFileMethodSyn::new(func.clone()),
				RouteFileMethod {
					route_info: RouteInfo::new(
						route_file.route_path.clone(),
						method,
					),
					meta,
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
	use beet_net::prelude::*;
	use beet_utils::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
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
		world.run_system_once(update_route_files).unwrap().unwrap();
		world.run_system_once(parse_route_file_rs).unwrap().unwrap();
		let file = world.entity(collection).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let route_method = world
			.entity(route)
			.get::<RouteFileMethod>()
			.unwrap()
			.clone();
		// send_wrapper::SendWrapper::assert_send(&tokens);
		route_method
			.meta
			.xpect()
			.to_be(RouteFileMethodMeta::Collection);
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
