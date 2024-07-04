use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn space_scene(mut commands: Commands) {
	commands.insert_resource(WrapAround::default());

	commands.spawn((
		AssetLoadBlockAppReady,
		Transform::from_translation(Vec3::new(0., 0., -1.))
			.with_scale(Vec3::splat(100.)),
		BundlePlaceholder::Sprite("space_background/Space_Stars2.png".into()),
		ImageScaleMode::Tiled {
			tile_x: true,
			tile_y: true,
			stretch_value: 0.01,
		},
	));
}
