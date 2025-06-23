use crate::prelude::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Serialized map of routes to all templates that need to be loaded on the client
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Deref)]
pub struct ClientIslandMap {
	islands: HashMap<RouteInfo, Vec<ClientIsland>>,
}


impl ClientIslandMap {
	pub fn new(islands: Vec<(RouteInfo, Vec<ClientIsland>)>) -> Self {
		let islands = islands
			.into_iter()
			.filter_map(|(route, islands)| {
				if islands.is_empty() {
					None
				} else {
					Some((route, islands))
				}
			})
			.collect();
		Self { islands }
	}

	pub fn write(&self, html_dir: &AbsPathBuf) -> Result {
		let path = html_dir.join("client_islands.ron");
		FsExt::write(&path, ron::ser::to_string(self)?)?;
		Ok(())
	}

	pub fn read(html_dir: &AbsPathBuf) -> Result<Self> {
		let path = html_dir.join("client_islands.ron");
		let content = ReadFile::to_string(&path)?;
		let islands: Self = ron::de::from_str(&content)?;
		Ok(islands)
	}
}
