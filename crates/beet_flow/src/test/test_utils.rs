use crate::prelude::*;
use anyhow::Result;
use bevy::asset::LoadState;
use bevy::prelude::*;
use sweet::*;

pub fn test_no_action_behavior_tree(world: &mut World) -> EntityTree {
	let entity = world
		.spawn(Running)
		.with_children(|parent| {
			parent.spawn_empty();
			parent.spawn_empty().with_children(|parent| {
				parent.spawn_empty();
			});
		})
		.id();
	EntityTree::new_with_world(entity, world)
}

type Func<T> = MockFunc<T, T, fn(T) -> T>;

pub fn observe_run_results(world: &mut World) -> Func<RunResult> {
	let func: Func<RunResult> = mock_func(|a| a);
	let func2 = func.clone();
	world.add_observer(move |on_result: Trigger<OnRunResult>| {
		func2.call(on_result.event().result());
	});
	func
}



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
