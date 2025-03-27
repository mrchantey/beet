use crate::prelude::*;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClientIslandMap {
	pub map: RapidHashMap<RouteInfo, Vec<ClientIsland>>,
}


#[cfg(feature = "parser")]
impl ClientIslandMap {
	pub fn into_mount_tokens(&self) -> proc_macro2::TokenStream {
		let items = self.map.iter().map(|(route, islands)| {
			let islands =
				islands.iter().map(|island| island.into_mount_tokens());
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[cfg(feature = "parser")]
	#[test]
	fn to_tokens() {
		use beet_rsx::rsx::ClientIsland;
		use beet_rsx::rsx::RustyTracker;

		let island = ClientIsland {
			tracker: RustyTracker::new(0, 0),
			type_name: "Foo".into(),
			ron: "bar".into(),
		};
		let island_tokens = island.into_mount_tokens();

		expect(
			ClientIslandMap {
				map: vec![(RouteInfo::new("/", "get"), vec![island.clone()])]
					.into_iter()
					.collect(),
			}
			.into_mount_tokens()
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
