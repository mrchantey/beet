use crate::prelude::*;
use anyhow::Result;
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
impl ClientIslandMap {
	pub fn into_mount_tokens(&self) -> proc_macro2::TokenStream {
		let items = self.map.iter().map(|(route, islands)| {
			let islands =
				islands.iter().map(|island| island.into_mount_tokens());
			let path_str = route.path.to_string_lossy();
			quote::quote! { (#path_str, Box::new(||{
				#( #islands )*
				Ok(())
			}) )}
		});
		quote::quote! {
			ClientIslandMoutFuncs::new(vec![#( #items ),*])
		}
	}
}

pub struct ClientIslandMountFuncs {
	pub map: RapidHashMap<&'static str, Box<dyn Fn() -> Result<()>>>,
}

impl ClientIslandMountFuncs {
	pub fn new(
		route_funcs: Vec<(&'static str, Box<dyn Fn() -> Result<()>>)>,
	) -> Self {
		Self {
			map: route_funcs.into_iter().collect(),
		}
	}

	#[cfg(target_arch = "wasm32")]
	pub fn mount(&self) -> Result<()> {}
}


impl IntoCollection<ClientIslandMountFuncs> for ClientIslandMountFuncs {
	fn into_collection(self) -> impl Collection {
		#[allow(unused)]
		|app: &mut AppRouter| {
			#[cfg(target_arch = "wasm32")]
			app.on_run_wasm.push(Box::new(|| self.mount()));
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
		use beet_rsx::rsx::TreeLocation;

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
			.into_mount_tokens()
			.to_string(),
		)
		.to_be(
			quote::quote! {
				ClientIslandMoutFuncs::new(vec![("/", Box::new(|| {
						beet::exports::ron::de::from_str::<Foo>(bar)?
							.pipe(RegisterEffects::new(TreeLocation::new(0u32,0u32,0u32)))?;
						Ok(())
				}))])
			}
			.to_string(),
		);
	}
}
