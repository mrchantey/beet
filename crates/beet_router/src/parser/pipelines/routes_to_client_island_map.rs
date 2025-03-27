use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;
use std::path::PathBuf;
use sweet::prelude::*;

#[derive(Debug)]
pub struct RoutesToClientIslandMap {
	pub islands_map_path: PathBuf,
	/// The [`CodegenFile::output`] path for the routes module.
	/// This will loaded, edited and saved again by the wasm routes codegen.
	// TODO probs cleaner to just use a seperate file
	pub codegen_output: PathBuf,
}


impl RoutesToClientIslandMap {
	pub fn new(routes_mod_path: impl Into<PathBuf>) -> Self {
		Self {
			codegen_output: routes_mod_path.into(),
			islands_map_path: Self::DEFAULT_ISLANDS_MAP_PATH.into(),
		}
	}
	pub fn new_with_islands_map_path(
		routes_mod_path: impl Into<PathBuf>,
		islands_map_path: impl Into<PathBuf>,
	) -> Self {
		Self {
			codegen_output: routes_mod_path.into(),
			islands_map_path: islands_map_path.into(),
		}
	}
}

impl RoutesToClientIslandMap {
	pub const DEFAULT_ISLANDS_MAP_PATH: &'static str =
		"target/client-islands.ron";
}

impl RsxPipeline<&Vec<(RouteInfo, RsxRoot)>, Result<()>>
	for RoutesToClientIslandMap
{
	fn apply(self, routes: &Vec<(RouteInfo, RsxRoot)>) -> Result<()> {
		let map: RapidHashMap<_, _> = routes
			.into_iter()
			.map(|(route, rsx)| {
				let islands = rsx.pipe(CollectClientIslands::default());
				(route.clone(), islands)
			})
			.collect();


		let ron = ron::ser::to_string_pretty(
			&ClientIslandMap {
				routes_mod_path: self.codegen_output,
				map,
			},
			Default::default(),
		)?;
		FsExt::write(&self.islands_map_path, &ron)?;

		Ok(())
	}
}
