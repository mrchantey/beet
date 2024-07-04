use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn space_background(mut commands: Commands) {
	let path = "space_background/Space_Stars2.png";

	commands.spawn((
		AssetLoadBlockAppReady,
		BundlePlaceholder::Sprite {
			path: path.to_string(),
			transform: Transform::from_translation(Vec3::new(0., 0., -1.))
				.with_scale(Vec3::splat(100.)),
		},
		ImageScaleMode::Tiled {
			tile_x: true,
			tile_y: true,
			stretch_value: 0.01,
		},
	));
}
