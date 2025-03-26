use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::Attribute;
use syn::File;
use syn::ItemFn;

/// edit the routes/mod.rs to include a #[wasm] collect func
pub struct CollectWasmRoutes {
	islands_map_path: PathBuf,
}

impl Default for CollectWasmRoutes {
	fn default() -> Self {
		Self {
			islands_map_path: PathBuf::from(
				RoutesToClientIslandMap::DEFAULT_ISLANDS_MAP_PATH,
			),
		}
	}
}

impl CollectWasmRoutes {
	/// Create a new instance of `CollectWasmRoutes` with a custom `islands_map_path`
	pub fn new(islands_map_path: impl Into<PathBuf>) -> Self {
		Self {
			islands_map_path: islands_map_path.into(),
		}
	}

	fn generate_collect_fn(islands_map: &ClientIslandMap) -> ItemFn {
		let tokens = islands_map.into_mount_tokens();
		syn::parse_quote! {
			#[cfg(target_arch = "wasm32")]
			pub fn collect() -> ClientIslandMountFuncs {
				#tokens
			}
		}
	}

	fn edit_file(file: &mut File, islands_map: &ClientIslandMap) {
		let target_wasm_attr: Attribute = syn::parse_quote! {
			#[cfg(target_arch = "wasm32")]
		};
		let current_func = file.items.iter_mut().find_map(|item| {
			if let syn::Item::Fn(func) = item {
				if func.sig.ident == "collect"
					&& func.attrs.iter().any(|attr| attr == &target_wasm_attr)
				{
					Some(func)
				} else {
					None
				}
			} else {
				None
			}
		});
		let new_fn = Self::generate_collect_fn(&islands_map);
		if let Some(func) = current_func {
			*func = new_fn;
		} else {
			file.items.push(new_fn.into());
		}
	}
}

impl BuildStep for CollectWasmRoutes {
	fn run(&self) -> Result<()> {
		let islands_map = ReadFile::to_bytes(&self.islands_map_path)?;
		let islands_map = ron::de::from_bytes::<ClientIslandMap>(&islands_map)?;
		let route_file = ReadFile::to_string(&islands_map.routes_mod_path)?;

		let mut file: File = syn::parse_file(&route_file)?;

		Self::edit_file(&mut file, &islands_map);

		let file = prettyplease::unparse(&file);

		FsExt::write(&islands_map.routes_mod_path, &file)?;

		Ok(())
	}
}


#[cfg(test)]
mod test {

	use crate::prelude::*;
	use http::Method;
	use quote::ToTokens;
	use sweet::prelude::*;

	use syn::File;

	use super::CollectWasmRoutes;

	fn island_map() -> ClientIslandMap {
		ClientIslandMap {
			routes_mod_path: "routes/mod.rs".into(),
			map: vec![(
				RouteInfo {
					path: "/".into(),
					method: Method::GET,
				},
				vec![],
			)]
			.into_iter()
			.collect(),
		}
	}

	#[test]
	fn empty() {
		let island_map = island_map();
		let island_map_tokens = island_map.into_mount_tokens();

		expect({
			let mut file: File = syn::parse_quote! {};
			CollectWasmRoutes::edit_file(&mut file, &island_map);
			file.to_token_stream().to_string()
		})
		.to_be(
			quote::quote! {
				#[cfg(target_arch = "wasm32")]
				pub fn collect() -> ClientIslandMountFuncs {
					#island_map_tokens
				}
			}
			.to_string(),
		);
	}
	#[test]
	fn with_both() {
		let island_map = island_map();
		let island_map_tokens = island_map.into_mount_tokens();

		expect({
			let mut file: File = syn::parse_quote! {
				#[cfg(not(target_arch = "wasm32"))]
				pub fn collect() -> Vec<Route> {
					todo!()
				}
				#[cfg(target_arch = "wasm32")]
				pub fn collect() -> ClientIslandMountFuncs {
					todo!()
				}
			};
			CollectWasmRoutes::edit_file(&mut file, &island_map);
			file.to_token_stream().to_string()
		})
		.to_be(
			quote::quote! {
				#[cfg(not(target_arch = "wasm32"))]
				pub fn collect() -> Vec<Route> {
					todo!()
				}
				#[cfg(target_arch = "wasm32")]
				pub fn collect() -> ClientIslandMountFuncs {
					#island_map_tokens
				}
			}
			.to_string(),
		);
	}
}
