use crate::prelude::*;
use anyhow::Result;
use std::path::PathBuf;
use sweet::prelude::*;

/// edit the routes/mod.rs to include a #[wasm] collect func
pub struct CollectWasmRoutes {
	islands_map_path: PathBuf,
}


impl Default for CollectWasmRoutes {
	fn default() -> Self {
		Self {
			islands_map_path: PathBuf::from(
				RoutesToClientIslandMap::DEFAULT_ISLANDS_MAP_PATH,
			),
		}
	}
}

impl CollectWasmRoutes {
	/// Create a new instance of `CollectWasmRoutes` with a custom `islands_map_path`
	pub fn new(islands_map_path: impl Into<PathBuf>) -> Self {
		Self {
			islands_map_path: islands_map_path.into(),
		}
	}

	pub fn run(&self) -> Result<()> {
		let islands_map = ReadFile::to_bytes(&self.islands_map_path)?;
		let islands_map = ron::de::from_bytes::<ClientIslandMap>(&islands_map)?;

		let route_file = ReadFile::to_bytes(&islands_map.routes_mod_path)?;
		Ok(())
	}
}
