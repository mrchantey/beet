use anyhow::Result;
use bevy::asset::LoadState;
use beet_core::prelude::*;

pub fn workspace_asset_plugin() -> AssetPlugin {
	AssetPlugin {
		file_path: "../../assets".into(),
		..default()
	}
}

pub fn block_on_asset_load<'a, A: Asset>(
	app: &'a mut App,
	path: &'static str,
) -> Result<Handle<A>> {
	let now = std::time::Instant::now();
	let handle = app
		.world_mut()
		.resource_mut::<AssetServer>()
		.load::<A>(path);
	loop {
		match app
			.world_mut()
			.resource_mut::<AssetServer>()
			.load_state(handle.id())
		{
			LoadState::Loaded => return Ok(handle),
			LoadState::Failed(err) => {
				anyhow::bail!("Asset load failed {:?}\n{}", path, err);
			}
			LoadState::NotLoaded => {
				anyhow::bail!("Asset not loaded {:?}", path);
			}
			LoadState::Loading => {
				// wait patiently 😴
			}
		}
		app.update();
		if now.elapsed().as_secs() > 10 {
			anyhow::bail!("Timeout: block_on_asset_load");
		}
	}
}
