use crate::prelude::*;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::scene::SceneInstanceReady;
use std::f32::consts::PI;

pub fn phone_texture_camera_3d(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		render_texture_bundle(&mut images, &mut materials),
		Camera3d::default(),
		// ClearColorConfig::Custom(Color::srgba(0., 0., 0., 0.)),
		Transform::from_xyz(0., 1.9, 3.),
	));
}


pub fn add_phone_render_texture_to_arm(
	trigger: Trigger<SceneInstanceReady>,
	names: Query<&Name>,
	children: Query<&Children>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	query: Single<&RenderTexture>,
) {
	let Some(phone_entity) =
		children.iter_descendants(trigger.entity()).find(|c| {
			names
				.get(*c)
				.map(|n| n.as_str() == "Phone")
				.unwrap_or(false)
		})
	else {
		return;
	};



	commands.entity(phone_entity).with_child((
		Name::new("Phone Texture"),
		Transform::from_xyz(0., 0.1, 0.).looking_to(Dir3::Z, Dir3::Y),
		Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(0.9)))),
		MeshMaterial3d(query.handle.clone()),
	));
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
		RenderLayers::layer(RENDER_TEXTURE_LAYER),
		// temp hack to get lights in render texture
		CascadeShadowConfigBuilder {
			first_cascade_far_bound: 20.0,
			maximum_distance: 40.0,
			..default()
		}
		.build(),
	));
}
