#[cfg(feature = "ml")]
pub mod ml;
mod templates;
use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::light::CascadeShadowConfigBuilder;
use std::f32::consts::PI;
pub use templates::*;

pub fn ui_terminal_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn ui_terminal(commands: Commands) { spawn_ui_terminal(commands, false); }

pub fn hello_world(mut commands: Commands) {
	commands.spawn((
		Name::new("Hello World Sequence"),
		CallOnSpawn::<(), Outcome>::default(),
		Sequence::new(),
		children![
			(Name::new("Hello"), EndWith(Outcome::PASS)),
			(Name::new("World"), EndWith(Outcome::PASS))
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
			shadow_maps_enabled: true,
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
