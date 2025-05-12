use beet_router::prelude::*;
use beet_rsx::prelude::*;
use proc_macro2::TokenStream;



pub fn client_island_map_into_mount_tokens(
	map: &ClientIslandMap,
) -> TokenStream {
	let items = map.map.iter().map(|(route, islands)| {
		let islands =
		islands.iter().map(island_into_mount_tokens);
		let path_str = route.path.to_string_lossy();
		quote::quote! { (#path_str, Box::new(||{
			#[allow(unused)]
			let tree_location_map = DomTarget::with(|dom| dom.tree_location_map().clone());
			#( #islands )*
			Ok(())
		}))}
	});
	quote::quote! {
		ClientIslandMountFuncs::new(vec![#( #items ),*])
	}
}



/// Convert the island into a token stream that can be used to mount the
/// island.
///
/// ## Panics
///
/// Panics if the type name or ron string are not valid tokens.
fn island_into_mount_tokens(island: &ClientIsland) -> TokenStream {
	let tracker_index = &island.tracker.index;
	let tracker_hash = &island.tracker.tokens_hash;
	let type_name = island.type_name.parse::<TokenStream>().unwrap();
	let ron = &island.ron;
	quote::quote! {
		// TODO resolve tracker to location
		beet::exports::ron::de::from_str::<#type_name>(#ron)?
			.into_node()
			// applying slots and removing lang nodes is a requirement of walking tree locations
			// consistently, it panics otherwise
			.xpipe(ApplySlots::default())?
			.xpipe(RemoveLangTemplates::default())
			.xpipe(RegisterEffects::new(
				tree_location_map.rusty_locations[
					&RustyTracker::new(#tracker_index,#tracker_hash)]
				)
			)?;
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_common::prelude::RustyTracker;
	use beet_router::types::RouteInfo;
	use sweet::prelude::*;


	#[test]
	fn island_to_tokens() {
		let island = ClientIsland {
			tracker: RustyTracker {
				index: 0,
				tokens_hash: 89,
			},
			type_name: "MyComponent".into(),
			ron: "(val:32)".into(),
		};
		expect(island_into_mount_tokens(&island).to_string()).to_be(
			quote::quote! {
				beet::exports::ron::de::from_str::<MyComponent>("(val:32)")?
					.into_node()
					.xpipe(ApplySlots::default())?
					.xpipe(RemoveLangTemplates::default())
					.xpipe(RegisterEffects::new(tree_location_map.rusty_locations[&RustyTracker::new(0u32,89u64)]))?;
			}
			.to_string(),
		);
	}


	#[test]
	fn island_map_to_tokens() {
		use beet_rsx::prelude::*;

		let island = ClientIsland {
			tracker: RustyTracker::new(0, 0),
			type_name: "Foo".into(),
			ron: "bar".into(),
		};
		let island_tokens = island_into_mount_tokens(&island);

		expect(
client_island_map_into_mount_tokens(&ClientIslandMap {
				map: vec![(RouteInfo::new("/", HttpMethod::Get), vec![island.clone()])]
					.into_iter()
					.collect(),
			})
			.to_string(),
		)
		.to_be(
			quote::quote! {
				ClientIslandMountFuncs::new(vec![("/", Box::new(|| {
					#[allow(unused)]
					let tree_location_map = DomTarget::with(|dom| dom.tree_location_map().clone());
					#island_tokens
					Ok(())
				}))])
			}
			.to_string(),
		);
	}
}
