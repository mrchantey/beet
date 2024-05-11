#![allow(unused)]
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use std::f32::consts::PI;

pub fn setup_scene_3d(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	// Camera
	commands.spawn(Camera3dBundle {
		transform: Transform::from_xyz(100.0, 100.0, 150.0)
			.looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
		..default()
	});

	// Plane
	commands.spawn(PbrBundle {
		mesh: meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0)),
		material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
		..default()
	});

	// Light
	commands.spawn(DirectionalLightBundle {
		transform: Transform::from_rotation(Quat::from_euler(
			EulerRot::ZYX,
			0.0,
			1.0,
			-PI / 4.,
		)),
		directional_light: DirectionalLight {
			shadows_enabled: true,
			..default()
		},
		cascade_shadow_config: CascadeShadowConfigBuilder {
			first_cascade_far_bound: 200.0,
			maximum_distance: 400.0,
			..default()
		}
		.into(),
		..default()
	});
}
