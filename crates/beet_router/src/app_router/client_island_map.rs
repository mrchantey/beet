use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
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


	// fn island_into_mount_tokens
}

pub struct ClientIslandMountFuncs {
	pub map:
		RapidHashMap<&'static str, Box<dyn Send + Sync + Fn() -> Result<()>>>,
}

impl ClientIslandMountFuncs {
	pub fn new(
		route_funcs: Vec<(
			&'static str,
			Box<dyn Send + Sync + Fn() -> Result<()>>,
		)>,
	) -> Self {
		Self {
			map: route_funcs.into_iter().collect(),
		}
	}

	#[cfg(target_arch = "wasm32")]
	pub fn mount(&self) -> Result<()> {
		DomTarget::set(BrowserDomTarget::default());

		let mut path =
			web_sys::window().unwrap().location().pathname().unwrap();
		if path.len() > 1 && path.ends_with('/') {
			path.pop();
		}

		if let Some(mount_fn) = self.map.get(path.as_str()) {
			mount_fn()?;
		} else {
			let received_paths = self.map.keys().collect::<Vec<_>>();
			anyhow::bail!(
				"No mount function found for path: {}\nreceived paths: {:?}",
				path,
				received_paths
			);
		}

		EventRegistry::initialize()?;
		Ok(())
	}
}


impl IntoCollection<ClientIslandMountFuncs> for ClientIslandMountFuncs {
	fn into_collection(self) -> impl Collection {
		#[allow(unused)]
		move |app: &mut AppRouter| {
			#[cfg(target_arch = "wasm32")]
			app.on_run_wasm.push(Box::new(move |_| self.mount()));
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
				routes_mod_path: "foobar".into(),
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
