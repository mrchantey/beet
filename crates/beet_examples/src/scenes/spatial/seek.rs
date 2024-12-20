use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;
use bevyhub::prelude::*;


pub fn seek(mut commands: Commands) {
	let target = commands
		.spawn((
			Name::new("Target"),
			FollowCursor2d,
			AssetLoadBlockAppReady,
			BundlePlaceholder::Sprite {
				path: "spaceship_pack/planet_6.png".into(),
				image_mode: default(),
			},
			Transform::from_translation(Vec3::new(200., 0., 0.)),
		))
		.id();

	commands
		.spawn((
			Name::new("Agent"),
			BundlePlaceholder::Sprite {
				path: "spaceship_pack/ship_2.png".into(),
				image_mode: default(),
			},
			AssetLoadBlockAppReady,
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_dist(500.),
			SteerTarget::Entity(target),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Seek"),
				RunOnAppReady::default(),
				TargetEntity(parent.parent_entity()),
				Seek::default(),
			));
		});
}
