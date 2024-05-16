#![allow(unused)]
use crate::follow_cursor_3d;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use forky_bevy::systems::close_on_esc;
use std::f32::consts::PI;

#[derive(Default)]
pub struct ExamplePlugin3d;

impl Plugin for ExamplePlugin3d {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						fit_canvas_to_parent: true,
						..default()
					}),
					..default()
				})
				.set(AssetPlugin {
					file_path: "/assets".into(),
					..default()
				})
				.build(),
		)
		.add_systems(Startup, setup_scene_3d)
		.add_systems(Update, (close_on_esc, follow_cursor_3d));
	}
}


pub fn setup_scene_3d(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	// Plane
	commands.spawn(PbrBundle {
		mesh: meshes.add(Plane3d::default().mesh().size(100., 100.)),
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
			first_cascade_far_bound: 20.0,
			maximum_distance: 40.0,
			..default()
		}
		.into(),
		..default()
	});
}
