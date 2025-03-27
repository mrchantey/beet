use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;
use std::path::PathBuf;
use sweet::prelude::*;

#[derive(Debug)]
pub struct RoutesToClientIslandMap {
	pub islands_map_path: PathBuf,
}

impl Default for RoutesToClientIslandMap {
	fn default() -> Self {
		Self {
			islands_map_path: Self::DEFAULT_ISLANDS_MAP_PATH.into(),
		}
	}
}


impl RoutesToClientIslandMap {
	pub fn new(island_map_path: impl Into<PathBuf>) -> Self {
		Self {
			islands_map_path: island_map_path.into(),
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
			&ClientIslandMap { map },
			Default::default(),
		)?;
		FsExt::write(&self.islands_map_path, &ron)?;

		Ok(())
	}
}
