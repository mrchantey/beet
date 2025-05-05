use crate::prelude::*;
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientIslandMap {
	pub map: RapidHashMap<RouteInfo, Vec<ClientIsland>>,
}
