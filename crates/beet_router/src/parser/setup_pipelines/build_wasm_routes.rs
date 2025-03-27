use crate::prelude::*;
use anyhow::Result;
use beet_rsx::rsx::BuildStep;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use sweet::prelude::*;
use syn::ItemFn;

/// edit the routes/mod.rs to include a #[wasm] collect func
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildWasmRoutes {
	islands_map_path: PathBuf,
	codegen_file: CodegenFile,
}

impl BuildWasmRoutes {
	pub fn new(out_file: impl Into<WorkspacePathBuf>, pkg_name: &str) -> Self {
		Self::new_with_options(
			out_file,
			pkg_name,
			RoutesToClientIslandMap::DEFAULT_ISLANDS_MAP_PATH.into(),
		)
	}


	pub fn new_with_options(
		out_file: impl Into<WorkspacePathBuf>,
		pkg_name: &str,
		islands_map_path: PathBuf,
	) -> Self {
		let output = out_file.into().into_canonical_unchecked();

		Self {
			islands_map_path,
			codegen_file: CodegenFile {
				output,
				pkg_name: Some(pkg_name.into()),
				..Default::default()
			},
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
