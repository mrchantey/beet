use anyhow::Result;
use beet_examples::beet::prelude::*;
use beet_examples::prelude::*;
use beet_examples::scenes;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
mod utils;
use rayon::prelude::*;



fn main() -> Result<()> {
	std::fs::remove_dir_all("target/scenes").ok();

	vec![Project {
		name: "beet-basics",
		scenes: vec![
			SceneItem::new("empty", || {}),
			SceneItem::new("camera-2d", scenes::camera_2d),
			SceneItem::new("camera-3d", scenes::camera_3d),
			SceneItem::new("beet-debug", scenes::beet_debug),
			// text
			SceneItem::new("ui-terminal", scenes::ui_terminal),
			SceneItem::new("ui-terminal-input", scenes::ui_terminal_input),
			SceneItem::new("hello-world", scenes::hello_world),
			SceneItem::new("hello-net", scenes::hello_net),
			SceneItem::new("sentence-selector", scenes::hello_ml),
			// 2d
			SceneItem::new("space-scene", scenes::space_scene),
			SceneItem::new("seek", scenes::seek),
			SceneItem::new("flock", scenes::flock),
			// 3d
			SceneItem::new("ground-3d", scenes::ground_3d),
			SceneItem::new("lighting-3d", scenes::lighting_3d),
			SceneItem::new("hello-animation", scenes::hello_animation),
			SceneItem::new("seek-3d", scenes::seek_3d),
			// fetch
			SceneItem::new("fetch-scene", scenes::fetch_scene),
			SceneItem::new("fetch-npc", scenes::fetch_npc),
			// frozen-lake
			SceneItem::new(
				"frozen-lake-scene",
				scenes::frozen_lake::frozen_lake_scene,
			),
			SceneItem::new(
				"frozen-lake-train",
				scenes::frozen_lake::frozen_lake_train,
			),
			SceneItem::new(
				"frozen-lake-run",
				scenes::frozen_lake::frozen_lake_run,
			),
		],
	}]
	.into_par_iter()
	.map(|project| project.save())
	.collect::<Result<Vec<_>>>()?;
	Ok(())
}

struct Project {
	pub name: &'static str,
	pub scenes: Vec<SceneItem>,
}

impl Project {
	pub fn save(self) -> Result<()> {
		self.scenes
			.into_iter()
			.map(|scene| scene.save(self.name))
			.collect::<Result<Vec<_>>>()?;
		Ok(())
	}
}

struct SceneItem {
	pub name: &'static str,
	pub system: SystemConfigs,
}

impl SceneItem {
	pub fn new<M>(
		name: &'static str,
		system: impl IntoSystemConfigs<M>,
	) -> Self {
		Self {
			name,
			system: system.into_configs(),
		}
	}
	pub fn save(self, _project_name: &str) -> Result<()> {
		let mut app = App::new();
		app.add_plugins((
			utils::MostDefaultPlugins,
			BeetPlugins::default(),
			ExamplePluginTypesFull,
		))
		.finish();

		Schedule::default()
			.add_systems(self.system)
			.run(app.world_mut());

		let filename = format!("target/scenes/{}.ron", self.name);

		save_scene(app.world_mut(), &filename)
	}
}


// mod reflect_utils{
//   use bevy::{core_pipeline::tonemapping::{DebandDither, Tonemapping}, prelude::Camera2d, reflect::Reflect, render::{camera::*, primitives::Frustum, view::VisibleEntities}, transform::components::{GlobalTransform, Transform}};

// 	#[derive(Reflect)]
// 	pub struct ReflectRoot{
// 		pub camera2d: Camera2dReflect,
// 	}

// 	#[derive(Reflect)]
// 	pub struct Camera2dReflect{
// 		pub camera: Camera,
// 		pub camera_render_graph: CameraRenderGraph,
// 		pub projection: OrthographicProjection,
// 		pub visible_entities: VisibleEntities,
// 		pub frustum: Frustum,
// 		pub transform: Transform,
// 		pub global_transform: GlobalTransform,
// 		pub camera_2d: Camera2d,
// 		pub tonemapping: Tonemapping,
// 		pub deband_dither: DebandDither,
// 		pub main_texture_usages: CameraMainTextureUsages,
// 	}
// }
