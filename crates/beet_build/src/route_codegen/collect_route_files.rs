use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;


/// Call [`CodegenFile::add_item`] for every [`RouteFileMethod`] in the
/// [`RouteFile`] children.
pub fn collect_route_files(
	mut query: Populated<
		(&mut CodegenFile, &RouteFileCollection, &Children),
		Added<CodegenFile>,
	>,
	route_files: Query<(&RouteSourceFile, &Children)>,
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

			for route_file_method in route_file_children
				.iter()
				.filter_map(|child| methods.get(child).ok())
			{
				let func_ident = &route_file_method.item.sig.ident;

				let endpoint = Endpoint::new_with(
					route_file_method.route_info.method,
					collection.category.cache_strategy(),
				)
				.self_token_stream();

				let is_async = route_file_method.item.sig.asyncness.is_some();
				let handler = match collection.category {
					RouteCollectionCategory::Pages => {
						// page routes are presumed to be bundles
						match is_async {
							true => quote! {
								RouteHandler::async_bundle(#endpoint, #mod_ident::#func_ident)
							},
							false => quote! {
								RouteHandler::bundle(#endpoint, #mod_ident::#func_ident)
							},
						}
					}
					RouteCollectionCategory::Actions => {
						let out_ty = match route_file_method.returns_result() {
							true => quote! { JsonResult },
							false => quote! { Json },
						};
						// Action routes may be any kind of route
						quote! {
							RouteHandler::action(
								#endpoint,
								#mod_ident::#func_ident.pipe(#out_ty::pipe)
							)
						}
					}
				};
				let filter =
					RouteFilter::new(&route_file_method.route_info.path)
						.self_token_stream();


				children.push(quote! {(
					#filter,
					#handler
				)});
			}
		}


		let collection_name = codegen_file.name();
		let collection_ident = quote::format_ident!("{collection_name}_routes");

		codegen_file.add_item::<syn::ItemFn>(parse_quote! {
			#[cfg(feature = "server")]
			pub fn #collection_ident() -> impl Bundle {
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
	use bevy::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::default());
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
