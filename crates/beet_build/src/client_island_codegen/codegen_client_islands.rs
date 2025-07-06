use crate::prelude::*;
use beet_bevy::prelude::When;
use beet_common::prelude::*;
use beet_router::prelude::ClientIslandMap;
use beet_rsx::prelude::*;
use bevy::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use serde::Serialize;
use syn::Type;

/// Marker for collecting client islands.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Component,
)]
pub struct CollectClientIslands {
	#[serde(flatten)]
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
	pub codegen: CodegenFile,
}


impl CollectClientIslands {
	pub fn load_islands_impl(island_map: &ClientIslandMap) -> TokenStream {
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


pub fn export_client_island_codegen(
	_: Query<(), Changed<RouteCodegenRoot>>,
	config: When<Res<WorkspaceConfig>>,
	mut query: Query<&mut CollectClientIslands>,
) -> Result {
	for mut collect_islands in query.iter_mut() {
		let client_island_map =
			ClientIslandMap::read(&config.client_islands_path.into_abs())?;

		let load_islands =
			CollectClientIslands::load_islands_impl(&client_island_map);

		collect_islands.codegen.clear_items();

		collect_islands.codegen.add_item::<syn::ItemStruct>(
			syn::parse_quote! {
				pub struct ClientIslandPlugin;
			},
		);
		collect_islands
			.codegen
			.add_item::<syn::ItemImpl>(syn::parse_quote! {
				impl Plugin for ClientIslandPlugin {
					fn build(&self, app: &mut App) {
						#load_islands
					}
				}
			});

		let num_islands: usize =
			client_island_map.values().map(|v| v.len()).sum();
		let num_routes = client_island_map.len();

		debug!(
			"Exporting {num_islands} client islands for {num_routes} routes",
		);

		collect_islands.codegen.build_and_write()?;
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

		CollectClientIslands::load_islands_impl(&map)
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
								TemplateSerde::parse::<beet_build::client_island_codegen::codegen_client_islands::test::Foo>("(7)")
								.unwrap()
								.into_node_bundle()
							));
							world.spawn((
								DomIdx(1u32),
								TemplateSerde::parse::<beet_build::client_island_codegen::codegen_client_islands::test::Foo>("(8)")
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
