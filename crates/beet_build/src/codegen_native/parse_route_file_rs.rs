use crate::prelude::*;
use beet_router::prelude::HttpMethod;
use beet_router::types::RouteInfo;
use beet_router::types::RoutePath;
use beet_utils::prelude::ReadFile;
use bevy::prelude::*;
use std::str::FromStr;
use syn::Visibility;



pub fn parse_route_file_rs(
	mut commands: Commands,
	query: Populated<(Entity, &RouteFile), Added<RouteFile>>,
) -> Result<()> {
	for (entity, route_file) in query.iter().filter(|(_, file)| {
		file.abs_path.extension().map_or(false, |ext| ext == "rs")
	}) {
		let mut parent = commands.entity(entity);

		let route_path = RoutePath::from_file_path(&route_file.local_path)?;
		let file_str = ReadFile::to_string(&route_file.abs_path)?;

		// collect all public functions, including handlers and
		// possibly their frontmatter
		let func_idents = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(func) = item {
					match &func.vis {
						Visibility::Public(_) => {
							let sig_str = func.sig.ident.to_string();
							return Some(sig_str);
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		for method in func_idents
			.iter()
			.filter_map(|ident_str| HttpMethod::from_str(ident_str).ok())
		{
			let method_config_ident =
				format!("config_{}", method.to_string().to_lowercase());
			let config = func_idents
				.iter()
				.find_map(|ident| match ident.as_str() {
					"config" => Some(RouteFileMethodConfig::File),
					ident if ident == &method_config_ident => {
						Some(RouteFileMethodConfig::Method)
					}
					_ => None,
				})
				.unwrap_or_default();

			parent.with_child(RouteFileMethod {
				route_info: RouteInfo::new(route_path.clone(), method),
				config,
			});
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_router::prelude::HttpMethod;
	use beet_router::types::RoutePath;
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
			.config
			.xpect()
			.to_be(RouteFileMethodConfig::FileGroup);
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
