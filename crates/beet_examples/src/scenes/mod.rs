#[cfg(feature = "ml")]
pub mod ml;
use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::light::CascadeShadowConfigBuilder;
use std::f32::consts::PI;


pub fn ui_terminal_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn ui_terminal(commands: Commands) { spawn_ui_terminal(commands, false); }


pub fn hello_world(mut commands: Commands) {
	commands.spawn((
		Name::new("Hello World Sequence"),
		TriggerDeferred::run(),
		Sequence::default(),
		children![
			(Name::new("Hello"), EndOnRun(SUCCESS)),
			(Name::new("World"), EndOnRun(SUCCESS))
		],
	));
}


pub fn camera_2d(mut commands: Commands) { commands.spawn(Camera2d); }

pub fn camera_3d(mut commands: Commands) {
	commands.spawn(Camera3d::default());
}

pub fn ground_3d(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(50.)))),
		MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
	));
}


pub fn lighting_3d(mut commands: Commands) {
	// Light
	commands.spawn((
		DirectionalLight {
			shadows_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(
			EulerRot::ZYX,
			0.0,
			1.0,
			-PI / 4.,
		)),
		CascadeShadowConfigBuilder {
			first_cascade_far_bound: 20.0,
			maximum_distance: 40.0,
			..default()
		}
		.build(),
	));
}


pub fn space_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((
		Transform::from_translation(Vec3::new(0., 0., -1.))
			.with_scale(Vec3::splat(100.)),
		Sprite {
			image: asset_server.load("space_background/Space_Stars2.png"),
			image_mode: SpriteImageMode::Tiled {
				tile_x: true,
				tile_y: true,
				stretch_value: 0.02,
			},
			..default()
		},
	));
}
