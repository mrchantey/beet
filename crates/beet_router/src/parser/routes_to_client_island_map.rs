use crate::prelude::*;
use anyhow::Result;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;
use sweet::prelude::*;

#[derive(Debug)]
pub struct RoutesToClientIslandMap {
	pub islands_map_path: AbsPathBuf,
}

impl Default for RoutesToClientIslandMap {
	fn default() -> Self {
		Self {
			islands_map_path: Self::default_islands_map_path(),
		}
	}
}


impl RoutesToClientIslandMap {
	pub fn new(islands_map_path: AbsPathBuf) -> Self {
		Self { islands_map_path }
	}
}

impl RoutesToClientIslandMap {
	pub fn default_islands_map_path() -> AbsPathBuf {
		WorkspacePathBuf::new("target/client-islands.ron")
			.into_canonical_unchecked()
	}
}

impl Pipeline<&Vec<(RouteInfo, RsxNode)>, Result<()>>
	for RoutesToClientIslandMap
{
	fn apply(self, routes: &Vec<(RouteInfo, RsxNode)>) -> Result<()> {
		let map: RapidHashMap<_, _> = routes
			.into_iter()
			.map(|(route, rsx)| {
				let islands = rsx.xpipe(CollectClientIslands::default());
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
