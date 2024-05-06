use beet::prelude::*;
use bevy::prelude::*;
use forky_bevy::systems::close_on_esc;
mod auto_spawn;
mod follow_cursor;
mod randomize_position;
mod render_text;
mod wrap_around;
pub use auto_spawn::*;
#[allow(unused_imports)]
pub use follow_cursor::*;
pub use randomize_position::*;
pub use render_text::*;
pub use wrap_around::*;
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
/// Boilerplate for examples
pub struct ExamplePlugin;

impl Plugin for ExamplePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(WrapAround::default())
			// .add_plugins(WorldInspectorPlugin::new())
			.add_plugins(DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					fit_canvas_to_parent: true,
					// resolution: window::WindowResolution::new(960., 960.),
					// position: WindowPosition::At(IVec2::new(5120, 0)),
					..default()
				}),
				..default()
			}))
			// .add_plugins(WorldInspectorPlugin::new())
			.add_plugins(DefaultBeetPlugins::default())
			.add_systems(Startup, space_setup)
			.add_systems(Update, follow_cursor::follow_cursor)
			.add_systems(Update, close_on_esc)
			// .add_systems(PreUpdate, auto_spawn::auto_spawn.before(PreTickSet))
			.add_systems(Update, randomize_position.in_set(PreTickSet))
			.add_systems(
				Update,
				(update_wrap_around, wrap_around)
					.chain()
					.run_if(|res: Option<Res<WrapAround>>| res.is_some())
					.in_set(PostTickSet),
			)
			.insert_resource(WrapAround::default());
		/*-*/



		let world = app.world_mut();

		world.init_component::<AutoSpawn>();
		world.init_component::<RandomizePosition>();
		world.init_component::<RenderText>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();
		registry.register::<AutoSpawn>();
		registry.register::<RandomizePosition>();
		registry.register::<RenderText>();
	}
}


fn space_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	// camera
	commands.spawn(Camera2dBundle::default());

	// background
	commands.spawn((
		SpriteBundle {
			texture: asset_server.load("space_background/Space_Stars2.png"),
			transform: Transform::from_translation(Vec3::new(0., 0., -1.))
				.with_scale(Vec3::splat(100.)),
			..default()
		},
		ImageScaleMode::Tiled {
			tile_x: true,
			tile_y: true,
			stretch_value: 0.01,
		},
	));
}
