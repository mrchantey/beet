use crate::prelude::*;
use beet_template::prelude::*;
use rapidhash::RapidHashMap;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ClientIslandMap {
	pub map: RapidHashMap<RouteInfo, Vec<ClientIsland>>,
}
