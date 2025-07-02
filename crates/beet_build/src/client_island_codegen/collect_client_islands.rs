use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_common::prelude::*;
use beet_router::prelude::ClientIslandMap;
use beet_template::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use serde::Serialize;
use syn::Type;

#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Component,
)]
pub struct CollectClientIslands;


impl CollectClientIslands {
	pub fn load_impl(island_map: &ClientIslandMap) -> TokenStream {
		let islands = island_map.iter().map(|(route_info, islands)| {
			let route_info = route_info.self_token_stream();
			let islands = islands.iter().map(|island| {
				let ty = syn::parse_str::<Type>(island.template.type_name())
					.unwrap();
				let ron = island.template.ron();
				let idx = island.dom_idx.self_token_stream();
				let mount_directive = if island.mount {
					quote! {ClientOnlyDirective,}
				} else {
					quote! {}
				};
				quote! {(
					#mount_directive
					#idx,
					TemplateSerde::parse::<#ty>(#ron)
						.unwrap()
						.into_node_bundle()
				)}
			});
			quote! {
					loader.try_mount(app, #route_info, |world| {
					#(world.spawn(#islands);)*
				});
			}
		});
		quote! {
			let loader = ClientIslandLoader::new();
			#(#islands)*
		}
	}
}


pub(super) fn collect_client_islands(
	config: When<Res<WorkspaceConfig>>,
	mut query: Populated<&mut CodegenFile, Added<CollectClientIslands>>,
) -> Result {
	for mut codegen_file in query.iter_mut() {
		let client_island_map =
			ClientIslandMap::read(&config.client_islands_path.into_abs())?;

		let islands_impl = CollectClientIslands::load_impl(&client_island_map);

		codegen_file.add_item::<syn::ItemStruct>(syn::parse_quote! {
			pub struct ClientIslandPlugin;
		});
		codegen_file.add_item::<syn::ItemImpl>(syn::parse_quote! {
			impl Plugin for ClientIslandPlugin {
				fn build(&self, app: &mut App) {
					#islands_impl
				}
			}
		});
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;
	use quote::quote;
	use serde::Serialize;
	use sweet::prelude::*;


	#[derive(Serialize)]
	struct Foo(pub u32);

	#[test]
	fn works() {
		let map = ClientIslandMap::new(vec![(RouteInfo::get("test"), vec![
			ClientIsland {
				template: TemplateSerde::new(&Foo(7)),
				dom_idx: DomIdx(0),
				mount: true,
			},
			ClientIsland {
				template: TemplateSerde::new(&Foo(8)),
				dom_idx: DomIdx(1),
				mount: false,
			},
		])]);

		CollectClientIslands::load_impl(&map)
			.to_string()
			.xpect()
			.to_be_str(
				quote! {
					let loader = ClientIslandLoader::new();
					loader.try_mount(app,
						RouteInfo {
							path: RoutePath(std::path::PathBuf::from("test")),
							method: HttpMethod::Get
						},
						|world| {
							world.spawn((
								ClientOnlyDirective,
								DomIdx(0u32),
								TemplateSerde::parse::<beet_build::client_island_codegen::collect_client_islands::test::Foo>("(7)")
								.unwrap()
								.into_node_bundle()
							));
							world.spawn((
								DomIdx(1u32),
								TemplateSerde::parse::<beet_build::client_island_codegen::collect_client_islands::test::Foo>("(8)")
									.unwrap()
									.into_node_bundle()
							));
						}
					);
				}
				.to_string(),
			);
	}
}
