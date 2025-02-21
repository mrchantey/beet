// pub mod flow;
// pub mod ml;
// pub mod spatial;

use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;


pub fn ui_terminal_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn ui_terminal(commands: Commands) { spawn_ui_terminal(commands, false); }


pub fn hello_world(mut commands: Commands) {
	commands
		.spawn((
			Name::new("Hello World Sequence"),
			RunOnSpawn::default(),
			Sequence::default(),
		))
		.with_children(|parent| {
			parent.spawn((Name::new("Hello"), ReturnWith(RunResult::Success)));
			parent.spawn((Name::new("World"), ReturnWith(RunResult::Success)));
		});
}


pub fn camera_2d(mut commands: Commands) { commands.spawn(Camera2d); }


pub fn space_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((
		Transform::from_translation(Vec3::new(0., 0., -1.))
			.with_scale(Vec3::splat(100.)),
		Sprite {
			image: asset_server.load("space_background/Space_Stars2.png"),
			image_mode: SpriteImageMode::Tiled {
				tile_x: true,
				tile_y: true,
				stretch_value: 0.01,
			},
			..default()
		},
	));
}
