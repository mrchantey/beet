use anyhow::Result;
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::time::TimePlugin;
use bevy::ui::UiPlugin;
use bevy::winit::WinitPlugin;

fn main() -> Result<()> {
	vec![Project {
		name: "beet-basics",
		scenes: vec![
			SceneItem::new("empty", || {}),
			SceneItem::new("log-to-ui", spawn_log_to_ui),
			SceneItem::new("hello-world", scenes::hello_world),
			SceneItem::new("hello-net", scenes::hello_net),
			SceneItem::new("sentence-selector", scenes::sentence_selector),
			// SceneItem::new("camera-2d", Camera2dBundle::default()),
			// SceneItem::new("camera-3d", Camera3dBundle::default()),
		],
	}]
	.into_iter()
	.map(|project| project.save())
	.collect::<Result<Vec<_>>>()?;
	Ok(())
}

mod scenes;

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

	pub fn save(self, project_name: &str) -> Result<()> {
		let mut app = App::new();
		app.add_plugins((
			//bevy
			TaskPoolPlugin::default(),
			HierarchyPlugin::default(),
			TransformPlugin::default(),
			AssetPlugin::default(),
			RenderPlugin::default(),
			UiPlugin::default(),
			//beet
			DefaultBeetPlugins::default(),
			// examples
			ExamplePlugins,
			// DefaultPlugins
			// .build()
			// .disable::<TimePlugin>()
			// .disable::<RenderPlugin>()
			// .disable::<TimePlugin>()
			// .disable::<InputPlugin>()
			// .disable::<WinitPlugin>()
			// .disable::<WindowPlugin>(),
			// TaskPoolPlugin::default(),
			// ExamplePlugin::default(),
		))
		// .register_type::<reflect_utils::ReflectRoot>()
		.finish();

		Schedule::default()
			.add_systems(self.system)
			.run(app.world_mut());

		let filename =
			format!("target/scenes/{}/{}.ron", project_name, self.name);

		save_scene(app.world_mut(), &filename)
	}
}

// fn bundle_to_system<B: Bundle>(bundle: fn() -> B) -> SystemConfigs {
// 	(move |mut commands: Commands| {
// 		commands.spawn(bundle());
// 	})
// 	.into_configs()
// }


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
