use crate::prelude::*;
use beet_common::prelude::IntoCustomTokens;
use beet_common::prelude::TempNonSendMarker;
use beet_net::prelude::*;
use bevy::prelude::*;
use syn::ItemFn;
use syn::parse_quote;


/// Call [`CodegenFile::add_item`] for every [`RouteFileMethod`] in the
/// [`RouteFile`] children.
pub fn collect_file_group(
	_: TempNonSendMarker,
	mut query: Populated<
		(&mut CodegenFileSendit, &FileGroupSendit, &Children),
		Added<FileGroupSendit>,
	>,
	route_files: Query<(&RouteFile, &Children)>,
	methods: Query<&RouteFileMethod>,
) {
	for (mut codegen_file, _file_group, file_group_children) in query.iter_mut()
	{
		let mut route_infos = Vec::<&RouteInfo>::new();
		let mut route_handlers = Vec::<syn::Path>::new();

		for (route_file, route_file_children) in file_group_children
			.iter()
			.filter_map(|child| route_files.get(child).ok())
		{
			codegen_file.add_item(route_file.item_mod());
			let mod_ident = route_file.mod_ident();

			for method in route_file_children
				.iter()
				.filter_map(|child| methods.get(child).ok())
			{
				route_infos.push(&method.route_info);
				let http_method = quote::format_ident!(
					"{}",
					method.route_info.method.to_string().to_lowercase()
				);
				route_handlers.push(parse_quote!(
					#mod_ident::#http_method
				));
			}
		}

		// TODO allow file group to specify axum router state type
		let state_ty: syn::Type = parse_quote!(());

		let route_infos = route_infos.into_custom_token_stream();
		codegen_file.add_item::<ItemFn>(parse_quote! {
			pub fn route_infos()-> Vec<RouteInfo> {
				#route_infos
			}
		});
		codegen_file.add_item::<ItemFn>(parse_quote! {
			pub fn router_plugin(router: Router<#state_ty>)-> Router<#state_ty> {
				#(router = beet_route(router, #route_handlers);)*
				router
			}
		});
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_bevy::prelude::WorldMutExt;
	use beet_parse::prelude::NodeTokensPlugin;
	use bevy::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((CodegenNativePlugin, NodeTokensPlugin));
		app.world_mut().spawn(FileGroup::test_site_docs());
		app.update();
		let codegen = app
			.world_mut()
			.query_filtered_once::<&CodegenFileSendit, With<FileGroupSendit>>()[0]
			.build_output()
			.unwrap()
			.to_token_stream()
			.to_string();
		// println!("{codegen}");
		expect(&codegen).to_contain(
			"pub fn route_infos () -> Vec < RouteInfo > { vec ! [RouteInfo {",
		);
		expect(&codegen).to_contain("mod route0 ;");
		expect(&codegen)
			.to_contain("router = beet_route (router , route0 :: get) ;");
	}
}
