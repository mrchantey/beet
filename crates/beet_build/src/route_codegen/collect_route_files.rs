use crate::prelude::*;
use beet_core::prelude::TokenizeSelf;
use beet_core::prelude::bevyhow;
use bevy::prelude::*;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;


/// Call [`CodegenFile::add_item`] for every [`RouteFileMethod`] in the
/// [`RouteFile`] children.
pub fn collect_route_files(
	mut query: Populated<
		(&mut CodegenFile, &RouteFileCollection, &Children),
		Changed<RouteFileCollection>,
	>,
	route_files: Query<(&RouteFile, &Children)>,
	methods: Query<&RouteFileMethod>,
) -> Result {
	for (mut codegen_file, collection, collection_children) in query.iter_mut()
	{
		let mut children = Vec::<TokenStream>::new();

		for (route_file, route_file_children) in collection_children
			.iter()
			.filter_map(|child| route_files.get(child).ok())
		{
			codegen_file.add_item(route_file.item_mod(collection.category));
			let mod_ident = route_file.mod_ident();

			for method in route_file_children
				.iter()
				.filter_map(|child| methods.get(child).ok())
			{
				let method_name =
					method.route_info.method.to_string_lowercase();

				let http_method = quote::format_ident!("{method_name}");
				let route_info = method.route_info.self_token_stream();

				let handler = match collection.category {
					RouteCollectionCategory::Pages => {
						// page routes are presumed to be bundles
						quote! {
								RouteHandler::new_bundle(#mod_ident::#http_method)
						}
					}
					RouteCollectionCategory::Actions => {
						// Action routes may be any kind of route
						quote! {
								RouteHandler::new(#mod_ident::#http_method)
						}
					}
				};
				let static_route =
					if collection.category.include_in_route_tree() {
						Some(quote! { StaticRoute, })
					} else {
						None
					};
				children.push(quote! {(
					#route_info,
					#static_route
					#handler
				)});
			}
		}


		let collection_name = codegen_file
			.output
			.file_stem()
			.map(|name| name.to_string_lossy().to_string())
			.ok_or_else(|| bevyhow!("failed"))?;


		let router_plugin_ident = quote::format_ident!(
			"{}Plugin",
			collection_name.to_upper_camel_case()
		);

		codegen_file.add_item::<syn::ItemFn>(parse_quote! {
			#[cfg(feature = "server")]
			#[allow(non_snake_case)]
			pub fn #router_plugin_ident() -> impl Bundle {
				children![#(#children),*]
			}
		});
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::WorldMutExt;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());
		app.world_mut().spawn(SourceFile::new(
			WsPathBuf::new(
				"crates/beet_router/src/test_site/test_docs/hello.md",
			)
			.into_abs(),
		));
		app.world_mut().spawn(RouteFileCollection::test_site_docs());
		app.update();
		app.world_mut()
			.query_filtered_once::<&CodegenFile, With<RouteFileCollection>>()[0]
			.build_output()
			.unwrap()
			.to_token_stream()
			.xpect()
			.to_be_snapshot();
	}
}
