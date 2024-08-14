use anyhow::Result;
use beet_examples::prelude::*;
use beetmash::prelude::*;
use bevy::ecs::observer::ObserverState;
use bevy::prelude::*;

const DIR: &str = "scenes";

fn plugin(app: &mut App) {
	app.add_plugins((MostDefaultPlugins, beet_example_plugin));
}

fn main() -> Result<()> {
	let checks = DynamicSceneChecks {
		resource_checks: false,
		entity_checks: true,
		component_checks: true,
		..Default::default()
	};


	SceneGroupExporter::new(plugin)
		.with_checks(checks.clone())
		.with_dir(DIR)
		.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
		)
		.add_scene("beet-debug", beet_examples::scenes::flow::beet_debug)
		.add_scene("hello-world", beet_examples::scenes::flow::hello_world)
		.export()?;

	SceneGroupExporter::new(plugin)
		.with_checks(checks.clone())
		.with_dir(DIR)
		.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
		)
		.without_clear_target()
		.add_scene("seek", beet_examples::scenes::spatial::seek)
		.add_scene("flock", beet_examples::scenes::spatial::flock)
		.add_scene("seek-3d", beet_examples::scenes::spatial::seek_3d)
		.add_scene(
			"hello-animation",
			beet_examples::scenes::spatial::hello_animation,
		)
		.export()?;


	SceneGroupExporter::new((plugin, plugin_ml))
		.with_checks(checks)
		.with_dir(DIR)
		.without_clear_target()
		.with_query::<(Without<ObserverState>, Without<Observer<OnLogMessage, ()>>)>(
		)
		.add_scene("hello-ml", beet_examples::scenes::ml::hello_ml)
		.add_scene("fetch-scene", beet_examples::scenes::ml::fetch_scene)
		.add_scene("fetch-npc", beet_examples::scenes::ml::fetch_npc)
		// frozen-lake
		.add_scene(
			"frozen-lake-scene",
			beet_examples::scenes::ml::frozen_lake_scene,
		)
		.add_scene(
			"frozen-lake-train",
			beet_examples::scenes::ml::frozen_lake_train,
		)
		.add_scene(
			"frozen-lake-run",
			beet_examples::scenes::ml::frozen_lake_run,
		)
		.export()?;
	Ok(())
}
