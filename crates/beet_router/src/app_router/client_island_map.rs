use crate::prelude::*;
use beet_rsx::rsx::ClientIsland;
use rapidhash::RapidHashMap;
use std::path::PathBuf;



#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ClientIslandMap {
	/// The path to the routes/mod.rs file at the root of these routes.
	/// This is used for editing the routes file.
	pub routes_mod_path: PathBuf,
	pub map: RapidHashMap<RouteInfo, Vec<ClientIsland>>,
}



#[cfg(feature = "parser")]
impl quote::ToTokens for ClientIslandMap {
	fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
		let routes_mod_path = &self.routes_mod_path.to_string_lossy();
		let items = self.map.iter().map(|(route, islands)| {
			let route = route.to_token_stream();
			let islands = islands.iter().map(|island| island.to_token_stream());
			quote::quote! { (#route,vec![#( #islands ),*]) }
		});
		tokens.extend(quote::quote! {
			ClientIslandMap {
				routes_mod_path: #routes_mod_path.into(),
				map: vec![#( #items ),*].into_iter().collect(),
			}
		});
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
		use beet_rsx::rsx::TreeLocation;
		use quote::ToTokens;

		expect(
			ClientIslandMap {
				routes_mod_path: "foobar".into(),
				map: vec![(RouteInfo::new("/", "get"), vec![ClientIsland {
					location: TreeLocation::new(0, 0, 0),
					type_name: "Foo".into(),
					ron: "bar".into(),
				}])]
				.into_iter()
				.collect(),
			}
			.to_token_stream()
			.to_string(),
		)
		.to_be(
			quote::quote! {
				ClientIslandMap {
					routes_mod_path: "foobar".into(),
					map: vec![(RouteInfo::new("/", "get"), vec![ClientIsland {
						location: TreeLocation::new(0u32, 0u32, 0u32),
						type_name: "Foo".into(),
						ron: "bar".into(),
					}])]
					.into_iter()
					.collect(),
				}
			}
			.to_string(),
		);
	}
}
