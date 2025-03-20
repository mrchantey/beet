use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use clap::Parser;
use rapidhash::RapidHashMap;
use std::path::PathBuf;
use sweet::prelude::*;


#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClientIslandMap(pub RapidHashMap<RouteInfo, Vec<ClientIsland>>);


#[derive(Debug, Parser)]
pub struct ExportClientIslands {
	#[arg(long, default_value = Self::DEFAULT_TEMPLATES_MAP_PATH)]
	pub islands_map_path: PathBuf,
}
impl ExportClientIslands {
	pub const DEFAULT_TEMPLATES_MAP_PATH: &'static str =
		"target/client-islands.ron";
}

impl RsxPipeline<&Vec<(RouteInfo, RsxRoot)>, Result<()>>
	for ExportClientIslands
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
			&ClientIslandMap(map),
			Default::default(),
		)?;
		FsExt::write(&self.islands_map_path, &ron)?;

		Ok(())
	}
}
