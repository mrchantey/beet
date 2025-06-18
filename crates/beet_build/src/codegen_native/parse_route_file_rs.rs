use crate::prelude::*;
use beet_common::prelude::TempNonSendMarker;
use beet_net::prelude::*;
use beet_utils::prelude::ReadFile;
use bevy::prelude::*;
use std::str::FromStr;
use syn::Visibility;



pub fn parse_route_file_rs(
	_: TempNonSendMarker, // spawning !send
	mut commands: Commands,
	query: Populated<(Entity, &RouteFile), Added<RouteFile>>,
) -> Result<()> {
	for (entity, route_file) in query.iter().filter(|(_, file)| {
		file.origin_path
			.extension()
			.map_or(false, |ext| ext == "rs")
	}) {
		let mut parent = commands.entity(entity);

		let file_str = ReadFile::to_string(&route_file.origin_path)?;

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

			parent.with_child((
				RouteFileMethodSyn::new(func.clone()).sendit(),
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
	use crate::prelude::*;
	use beet_net::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let group = world.spawn(FileGroup::test_site_pages()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world.run_system_once(parse_route_file_rs).unwrap().unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
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
			.to_be(RouteFileMethodMeta::FileGroup);
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
