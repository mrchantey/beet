use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::view::RenderLayers;




#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct RenderTexture {
	pub handle: Handle<StandardMaterial>,
}


pub const RENDER_TEXTURE_LAYER: usize = 1;


pub fn create_render_camera(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let size = Extent3d {
		width: 512,
		height: 512,
		..default()
	};

	// This is the texture that will be rendered to.
	let mut image = Image {
		texture_descriptor: TextureDescriptor {
			label: None,
			size,
			dimension: TextureDimension::D2,
			format: TextureFormat::Bgra8UnormSrgb,
			mip_level_count: 1,
			sample_count: 1,
			usage: TextureUsages::TEXTURE_BINDING
				| TextureUsages::COPY_DST
				| TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		},
		..default()
	};

	// fill image.data with zeroes
	image.resize(size);

	let image_handle = images.add(image);
	let material_handle = materials.add(StandardMaterial {
		base_color_texture: Some(image_handle.clone()),
		reflectance: 0.02,
		unlit: false,
		..default()
	});

	commands.spawn((
		Camera2d,
		Camera {
			order: -1,
			target: image_handle.into(),
			clear_color: Color::WHITE.into(),
			..default()
		},
		RenderLayers::layer(RENDER_TEXTURE_LAYER),
		material_handle,
	));
}
