use crate::prelude::*;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

pub fn phone_texture(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
	let layout =
		TextureAtlasLayout::from_grid(UVec2::splat(72), 16, 10, None, None);
	let texture_atlas_layout = texture_atlas_layouts.add(layout);
	commands.spawn((
		Name::new("PhoneTexture"),
		SpriteBundle {
			transform: Transform::from_scale(Vec3::splat(10.)),
			// transform: Transform::default().with_scale(Vec3::splat(10.)),
			texture: asset_server.load("openmoji/smileys-emotion.png"),
			..default()
		},
		TextureAtlas {
			layout: texture_atlas_layout,
			index: 0,
		},
		RenderLayers::layer(RENDER_TEXTURE_LAYER),
	));
}

pub fn phone_texture_camera(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		render_texture_bundle(&mut images, &mut materials),
		// Camera3d::default(),
		Camera2d,
	));
}
