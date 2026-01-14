use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::unbounded_related;
use beet_router::prelude::*;
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

				let path_pattern = PathPattern::new(&route_file_method.path)?;

				let method = route_file_method.method.self_token_stream();
				let is_async = route_file_method.item.sig.asyncness.is_some();
				let annoying_generics = match route_file_method.returns_result()
				{
					true => {
						quote! { ::<_,_,_,_,(SerdeResultIntoServerActionOut,_)> }
					}
					false => quote! {},
				};


				let mut builder_tokens = match (collection.category, is_async) {
					(RouteCollectionCategory::Pages, _) => {
						let mut items = vec![
							quote!(EndpointBuilder::new(#mod_ident::#func_ident)
								.with_method(#method)
								.with_content_type(ContentType::Html)),
						];
						// ssr check TODO very brittle
						if path_pattern.is_static()
							&& collection.category.cache_strategy()
								== CacheStrategy::Static
							&& route_file_method.method == HttpMethod::Get
						{
							items.push(
								quote!(.with_predicate(common_predicates::is_ssr())),
							);
						};
						items
					}
					(RouteCollectionCategory::Actions, true) => {
						vec![
							quote!(ServerAction::new_async #annoying_generics(#method, #mod_ident::#func_ident)
								.with_content_type(ContentType::Json)),
						]
					}
					(RouteCollectionCategory::Actions, false) => {
						vec![
							quote!(ServerAction::new #annoying_generics(#method, #mod_ident::#func_ident)
								.with_content_type(ContentType::Json)),
						]
					}
				};
				let path = route_file_method.path.to_string();
				builder_tokens.push(quote!(.with_path(#path)));
				let cache_strategy =
					collection.category.cache_strategy().self_token_stream();
				builder_tokens
					.push(quote!(.with_cache_strategy(#cache_strategy)));

				children.push(quote! {#(#builder_tokens)*});
			}
		}


		let collection_name = codegen_file.name();
		let collection_ident = quote::format_ident!("{collection_name}_routes");
		let bundle = unbounded_related::<Children>(children)?;

		codegen_file.add_item::<syn::ItemFn>(parse_quote! {
			#[cfg(feature = "server")]
			pub fn #collection_ident() -> impl Bundle {
				(InfallibleSequence, #bundle)
			}
		});
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use quote::ToTokens;

	#[test]
	fn works() {
		let mut world = BuildPlugin::world();
		world.spawn(RouteFileCollection::test_site_docs());
		world.run_schedule(ParseSourceFiles);
		world.query_filtered_once::<&CodegenFile, With<RouteFileCollection>>()
			[0]
		.build_output()
		.unwrap()
		.to_token_stream()
		.xpect_snapshot();
	}
}
