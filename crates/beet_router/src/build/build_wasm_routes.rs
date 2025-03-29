use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use sweet::prelude::*;
use syn::ItemFn;

/// Create a rust file that collects all the island mount functions
#[derive(Debug, Clone)]
pub struct BuildWasmRoutes {
	islands_map_path: CanonicalPathBuf,
	codegen_file: CodegenFile,
}

impl BuildWasmRoutes {
	pub fn new(codegen_file: CodegenFile) -> Self {
		Self {
			codegen_file,
			islands_map_path: RoutesToClientIslandMap::default_islands_map_path(
			),
		}
	}

	fn collect_fn(islands_map: &ClientIslandMap) -> ItemFn {
		let tokens = islands_map.into_mount_tokens();
		syn::parse_quote! {
			/// Collect all the island mount functions. The exact func used
			/// will be determined by the `window.location`
			pub fn collect() -> ClientIslandMountFuncs {
				#tokens
			}
		}
	}
}

impl BuildStep for BuildWasmRoutes {
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

	use super::BuildWasmRoutes;
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
			BuildWasmRoutes::collect_fn(&island_map)
				.to_token_stream()
				.to_string(),
		)
		.to_be(
			quote::quote! {
				/// Collect all the island mount functions. The exact func used
				/// will be determined by the `window.location`
				pub fn collect() -> ClientIslandMountFuncs {
					#island_map_tokens
				}
			}
			.to_string(),
		);
	}
}
