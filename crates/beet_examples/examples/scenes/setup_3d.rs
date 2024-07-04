use beet_examples::prelude::*;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use std::f32::consts::PI;


pub fn setup_ground_3d(mut commands: Commands) {
	commands.spawn(BundlePlaceholder::Pbr {
		mesh: MeshPlaceholder::Plane3d {
			plane: Plane3d::default(),
			width: 100.,
			height: 100.,
		},
		material: MaterialPlaceholder::Color(Color::srgb(0.3, 0.5, 0.3)),
	});
}


pub fn setup_lighting_3d(mut commands: Commands) {
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
