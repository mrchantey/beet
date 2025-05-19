use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::*;
use sweet::prelude::*;
use syn::ItemFn;

/// Create a rust file that collects all the island mount functions
/// This  step depends on the native step, we need to actually build
/// the rsx tree to find the client islands and create the IslandMap
#[derive(Debug, Clone)]
pub struct BuildWasmRoutes {
	islands_map_path: AbsPathBuf,
}
impl Default for BuildWasmRoutes {
	fn default() -> Self {
		Self {
			islands_map_path: default_paths::client_islands(),
		}
	}
}

impl BuildWasmRoutes {
	fn collect_fn(island_map: &ClientIslandMap) -> ItemFn {
		let tokens = client_island_map_into_mount_tokens(&island_map);

		syn::parse_quote! {
			/// Collect all the island mount functions. The exact func used
			/// will be determined by the `window.location`
			#[allow(dead_code)]
			pub fn collect() -> ClientIslandMountFuncs {
				#tokens
			}
		}
	}
}

impl Pipeline<CodegenFile, Result<()>> for BuildWasmRoutes {
	fn apply(self, mut codegen_file: CodegenFile) -> Result<()> {
		let islands_map = ReadFile::to_bytes(&self.islands_map_path)?;
		let islands_map = ron::de::from_bytes::<ClientIslandMap>(&islands_map)?;
		codegen_file.add_item(Self::collect_fn(&islands_map));
		codegen_file.build_and_write()?;
		Ok(())
	}
}

#[cfg(test)]
mod test {

	use super::BuildWasmRoutes;
	use crate::prelude::*;
	use beet_router::prelude::*;
	use quote::ToTokens;
	use sweet::prelude::*;

	fn island_map() -> ClientIslandMap {
		ClientIslandMap {
			map: vec![(
				RouteInfo {
					path: "/".into(),
					method: HttpMethod::Get,
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
		let island_map_tokens =
			client_island_map_into_mount_tokens(&island_map);

		expect(
			BuildWasmRoutes::collect_fn(&island_map)
				.to_token_stream()
				.to_string(),
		)
		.to_be(
			quote::quote! {
				/// Collect all the island mount functions. The exact func used
				/// will be determined by the `window.location`
				#[allow(dead_code)]
				pub fn collect() -> ClientIslandMountFuncs {
					#island_map_tokens
				}
			}
			.to_string(),
		);
	}
}
