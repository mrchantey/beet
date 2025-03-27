use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::ItemFn;

/// edit the routes/mod.rs to include a #[wasm] collect func
#[derive(Debug, Serialize, Deserialize)]
pub struct CollectWasmRoutes {
	islands_map_path: PathBuf,
	codegen_file: CodegenFile,
}

impl Default for CollectWasmRoutes {
	fn default() -> Self {
		Self {
			islands_map_path: PathBuf::from(
				RoutesToClientIslandMap::DEFAULT_ISLANDS_MAP_PATH,
			),
			codegen_file: CodegenFile::default(),
		}
	}
}

impl CollectWasmRoutes {
	fn collect_fn(islands_map: &ClientIslandMap) -> ItemFn {
		let tokens = islands_map.into_mount_tokens();
		syn::parse_quote! {
			#[cfg(target_arch = "wasm32")]
			pub fn collect() -> ClientIslandMountFuncs {
				#tokens
			}
		}
	}
}

impl BuildStep for CollectWasmRoutes {
	fn run(&self) -> Result<()> {
		let islands_map = ReadFile::to_bytes(&self.islands_map_path)?;
		let islands_map = ron::de::from_bytes::<ClientIslandMap>(&islands_map)?;

		let mut file = self.codegen_file.clone();
		file.add_item(Self::collect_fn(&islands_map));
		file.build_and_write()?;
		Ok(())
	}
}


#[cfg(test)]
mod test {

	use super::CollectWasmRoutes;
	use crate::prelude::*;
	use http::Method;
	use quote::ToTokens;
	use sweet::prelude::*;

	fn island_map() -> ClientIslandMap {
		ClientIslandMap {
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
	fn test_output() {
		let island_map = island_map();
		let island_map_tokens = island_map.into_mount_tokens();

		expect(
			CollectWasmRoutes::collect_fn(&island_map)
				.to_token_stream()
				.to_string(),
		)
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
}
