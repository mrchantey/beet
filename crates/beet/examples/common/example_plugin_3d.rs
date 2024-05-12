#![allow(unused)]
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use forky_bevy::systems::close_on_esc;
use std::f32::consts::PI;

pub struct ExamplePlugin3d;

impl Plugin for ExamplePlugin3d {
	fn build(&self, app: &mut App) {
		app.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				fit_canvas_to_parent: true,
				..default()
			}),
			..default()
		}))
		.add_systems(Startup, setup_scene_3d)
		.add_systems(Update, close_on_esc);
	}
}


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
