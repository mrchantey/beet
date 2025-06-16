use crate::prelude::*;
use beet_bevy::bevyhow;
use beet_common::prelude::TempNonSendMarker;
use beet_common::prelude::TokenizeSelf;
use beet_net::prelude::*;
use bevy::prelude::*;
use heck::ToUpperCamelCase;
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
) -> Result {
	for (mut codegen_file, file_group, file_group_children) in query.iter_mut()
	{
		let mut route_infos = Vec::<&RouteInfo>::new();
		let mut route_handlers = Vec::<syn::Expr>::new();

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
				let route_info = &method.route_info.self_token_stream();
				route_handlers.push(parse_quote!(
					(#route_info, #mod_ident::#http_method)
				));
			}
		}

		let route_infos = route_infos.self_token_stream();
		codegen_file.add_item::<ItemFn>(parse_quote! {
			pub fn route_infos()-> Vec<RouteInfo> {
				#route_infos
			}
		});

		let group_name = if let Some(group_name) = &file_group.group_name {
			group_name.clone()
		} else {
			codegen_file
				.output
				.file_stem()
				.map(|name| name.to_string_lossy().to_string())
				.ok_or_else(|| bevyhow!("failed"))?
		};

		let router_plugin_ident = quote::format_ident!(
			"{}RouterPlugin",
			group_name.to_upper_camel_case()
		);

		codegen_file.add_item::<syn::ItemStruct>(parse_quote! {
			pub struct #router_plugin_ident;
		});

		let meta_ty = &file_group.meta_type;
		let router_state_type = &file_group.router_state_type;
		codegen_file.add_item::<syn::ItemImpl>(parse_quote! {
			impl RouterPlugin for #router_plugin_ident {
				type State = #router_state_type;
				type Meta = #meta_ty;
				fn build(self, mut router: beet::exports::axum::Router<#router_state_type>)
					-> beet::exports::axum::Router<#router_state_type> {
					#(router = self.add_route(router, #route_handlers);)*
					router
				}
			}
		});
	}
	Ok(())
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
		expect(&codegen).to_contain("use crate as test_site");
		expect(&codegen).to_contain(
			"pub fn route_infos () -> Vec < RouteInfo > { vec ! [RouteInfo {",
		);
		expect(&codegen).to_contain("mod route0 ;");
		expect(&codegen)
			.to_contain("router = self . add_route (router , route0 :: get)");
	}
}
