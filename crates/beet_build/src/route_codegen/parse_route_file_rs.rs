//! Rust source file parsing for route extraction.
//!
//! This module handles parsing Rust source files to extract public functions
//! that match HTTP method names (get, post, put, delete, etc.) and creating
//! corresponding [`RouteFileMethod`] entities.

use crate::prelude::*;
use beet_core::prelude::*;
use std::str::FromStr;
use syn::Visibility;


/// Parses Rust source files to extract route handler functions.
///
/// This system scans each `.rs` file in a [`RouteFileCollection`] for public
/// functions whose names match HTTP methods (get, post, etc.) and spawns
/// [`RouteFileMethod`] entities for each one.
pub(crate) fn parse_route_file_rs(
	mut commands: Commands,
	query: Populated<
		(Entity, &SourceFile, &RouteSourceFile),
		Added<SourceFile>,
	>,
) -> Result {
	for (entity, source_file, route_file) in
		query.iter().filter(|(_, _, route_file)| {
			route_file
				.source_file_collection_rel
				.extension()
				.map_or(false, |ext| ext == "rs")
		}) {
		let file_str = fs_ext::read_to_string(&source_file)?;

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
				ChildOf(entity),
				RouteFileMethod::new_with(
					route_file.route_path.clone(),
					method,
					func,
				),
			));
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::super::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let collection =
			world.spawn(RouteFileCollection::test_site_pages()).id();
		world
			.run_system_cached::<Result, _, _>(create_route_files)
			.unwrap()
			.unwrap();
		world
			.run_system_cached::<Result, _, _>(parse_route_file_rs)
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
		route_method.method.xpect_eq(HttpMethod::Get);
		route_method.path.xpect_eq(RoutePath::new("/docs"));
	}
}
