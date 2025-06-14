use crate::prelude::*;
use beet_router::prelude::HttpMethod;
use beet_router::types::RouteInfo;
use beet_router::types::RoutePath;
use beet_utils::prelude::ReadFile;
use bevy::prelude::*;
use proc_macro2::Span;
use std::str::FromStr;
use syn::Ident;
use syn::Visibility;



pub fn parse_route_file_rust(
	mut commands: Commands,
	query: Populated<(Entity, &RouteFile), Added<RouteFile>>,
) -> Result<()> {
	for (
		entity,
		RouteFile {
			index,
			abs_path,
			local_path,
		},
	) in query.iter().filter(|(_, file)| {
		file.abs_path.extension().map_or(false, |ext| ext == "rs")
	}) {
		let mod_ident =
			Ident::new(&format!("route{}", index), Span::call_site());

		let mut parent = commands.entity(entity);

		let route_path = RoutePath::from_file_path(&local_path)?;
		let file_str = ReadFile::to_string(&abs_path)?;

		// collect all public functions, including handlers and
		// possibly their frontmatter
		let pub_funcs = syn::parse_file(&file_str)?
			.items
			.into_iter()
			.filter_map(|item| {
				if let syn::Item::Fn(func) = item {
					match &func.vis {
						Visibility::Public(_) => {
							let sig_str = func.sig.ident.to_string();
							return Some((sig_str, func));
						}
						_ => {}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		for (ident_str, func) in pub_funcs.iter().filter(|(ident_str, _)| {
			HttpMethod::METHODS.iter().any(|m| m == ident_str)
		}) {
			let frontmatter_ident = format!("{ident_str}_frontmatter");
			let frontmatter = match pub_funcs
				.iter()
				.find(|(s, _)| s == "frontmatter" || s == &frontmatter_ident)
			{
				Some((_, frontmatter_ident)) => {
					syn::parse_quote!({
						#mod_ident::#frontmatter_ident()
					})
				}
				None => syn::parse_quote!({ Default::default() }),
			};

			parent.with_child(
				FileRouteTokens {
					mod_ident: mod_ident.clone(),
					mod_import: ModImport::Path,
					abs_path: abs_path.clone(),
					local_path: local_path.clone(),
					route_info: RouteInfo {
						path: route_path.clone(),
						// we just checked its a valid method
						method: HttpMethod::from_str(&ident_str).unwrap(),
					},
					frontmatter,
					item_fn: func.clone(),
				}
				.sendit(),
			);
		}
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

		let group = world.spawn(FileGroup::test_site_pages()).id();
		world.run_system_once(spawn_route_files).unwrap().unwrap();
		world
			.run_system_once(parse_route_file_rust)
			.unwrap()
			.unwrap();
		let file = world.entity(group).get::<Children>().unwrap()[0];
		let route = world.entity(file).get::<Children>().unwrap()[0];
		let tokens = world
			.entity(route)
			.get::<FileRouteTokensSend>()
			.unwrap()
			.deref();
		// send_wrapper::SendWrapper::assert_send(&tokens);
		tokens
			.item_fn
			.to_token_stream()
			.to_string()
			.replace(" ", "")
			.xpect()
			.to_be(
				quote! {
				pub fn get() -> WebNode {
					rsx! {
						<PageLayout style: cascade title="foobar">
							party time!
						</PageLayout>
					}
				}
				}
				.to_string()
				.replace(" ", ""),
			);
	}
}
