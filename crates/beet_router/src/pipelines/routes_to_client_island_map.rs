use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use clap::Parser;
use rapidhash::RapidHashMap;
use std::path::PathBuf;
use sweet::prelude::*;


#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientIslandMap {
	/// The path to the routes/mod.rs file at the root of these routes.
	/// This is used for editing the routes file.
	pub routes_mod_path: PathBuf,
	pub map: RapidHashMap<RouteInfo, Vec<ClientIsland>>,
}



#[derive(Debug, Parser)]
pub struct RoutesToClientIslandMap {
	#[arg(long, default_value = Self::DEFAULT_ISLANDS_MAP_PATH)]
	pub islands_map_path: PathBuf,
	pub routes_mod_path: PathBuf,
}


impl RoutesToClientIslandMap {
	pub fn new(routes_mod_path: impl Into<PathBuf>) -> Self {
		Self {
			routes_mod_path: routes_mod_path.into(),
			islands_map_path: Self::DEFAULT_ISLANDS_MAP_PATH.into(),
		}
	}
	pub fn new_with_islands_map_path(
		routes_mod_path: impl Into<PathBuf>,
		islands_map_path: impl Into<PathBuf>,
	) -> Self {
		Self {
			routes_mod_path: routes_mod_path.into(),
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
				routes_mod_path: self.routes_mod_path,
				map,
			},
			Default::default(),
		)?;
		FsExt::write(&self.islands_map_path, &ron)?;

		Ok(())
	}
}
