use beet_net::prelude::*;
use bevy::platform::collections::HashMap;
use serde::Deserialize;
use serde::Serialize;
use crate::prelude::*;

/// Serialized map of routes to all templates that need to be loaded on the client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIslandMap {
	pub map: HashMap<RouteInfo, ClientIsland>,
}


