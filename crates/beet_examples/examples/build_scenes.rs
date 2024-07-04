use anyhow::Result;
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::ui::UiPlugin;

fn main() -> Result<()> {
	vec![Project {
		name: "beet-basics",
		scenes: vec![
			SceneItem::new("empty", || {}),
			SceneItem::new_bundle("camera-2d", BundlePlaceholder::Camera2d),
			SceneItem::new_bundle("camera-3d", BundlePlaceholder::Camera3d),
			SceneItem::new_resource("beet-debug", BeetDebugConfig::default()),
			SceneItem::new("space-background", scenes::space_background),
			SceneItem::new("ui-terminal", spawn_ui_terminal),
			SceneItem::new("seek", scenes::seek),
			SceneItem::new("flock", scenes::flock),
			SceneItem::new("hello-world", scenes::hello_world),
			SceneItem::new("hello-net", scenes::hello_net),
			SceneItem::new("sentence-selector", scenes::sentence_selector),
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
	pub fn new_bundle(name: &'static str, bundle: impl Bundle + Clone) -> Self {
		Self {
			name,
			system: (move |mut commands: Commands| {
				commands.spawn(bundle.clone());
			})
			.into_configs(),
		}
	}
	pub fn new_resource(
		name: &'static str,
		resource: impl Resource + Clone,
	) -> Self {
		Self {
			name,
			system: (move |mut commands: Commands| {
				commands.insert_resource(resource.clone());
			})
			.into_configs(),
		}
	}

	pub fn save(self, _project_name: &str) -> Result<()> {
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
		.register_type::<Name>()
		.register_type::<ImageScaleMode>()
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
