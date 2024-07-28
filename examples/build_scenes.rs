use anyhow::Result;
use beetmash::prelude::*;
use bevy::ecs::observer::ObserverState;
use bevy::prelude::*;
use std::fs;

fn main() -> Result<()> {
	std::fs::remove_dir_all("crates/beet_flow/scenes").ok();
	std::fs::remove_dir_all("crates/beetmash_core/scenes").ok();



	BeetmashSceneBuilder::new((
		MostDefaultPlugins,
		DefaultPlaceholderPlugin,
		UiTerminalPlugin,
	))
	.with_dir("crates/beet_flow/scenes")
	.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
	)
	.add_scene("beet-debug", beet::flow::scenes::beet_debug)
	.add_scene("hello-world", beet::flow::scenes::hello_world)
	.build()?;

	BeetmashSceneBuilder::new((
		MostDefaultPlugins,
		DefaultPlaceholderPlugin,
		UiTerminalPlugin,
	))
	.with_dir("crates/beet_spatial/scenes")
	.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
	)
	.add_scene("seek", beet::spatial::scenes::seek)
	.add_scene("flock", beet::spatial::scenes::flock)
	.add_scene("seek-3d", beet::spatial::scenes::seek_3d)
	.add_scene("hello-animation", beet::spatial::scenes::hello_animation)
	.build()?;


	BeetmashSceneBuilder::new((
		MostDefaultPlugins,
		DefaultPlaceholderPlugin,
		UiTerminalPlugin,
	))
	.with_dir("crates/beet_ml/scenes")
	.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
	)
	.add_scene("fetch-scene", beet::ml::scenes::fetch_scene)
	.add_scene("fetch-npc", beet::ml::scenes::fetch_npc)
	.add_scene("sentence-selector", beet::ml::scenes::hello_ml)
	// frozen-lake
	.add_scene(
		"frozen-lake-scene",
		beet::ml::scenes::frozen_lake::frozen_lake_scene,
	)
	.add_scene(
		"frozen-lake-train",
		beet::ml::scenes::frozen_lake::frozen_lake_train,
	)
	.add_scene(
		"frozen-lake-run",
		beet::ml::scenes::frozen_lake::frozen_lake_run,
	)
	.build()?;
	Ok(())
}
