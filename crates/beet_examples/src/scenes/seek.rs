use beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;




pub fn seek(mut commands: Commands) {
	// target
	let target = commands
		.spawn((
			FollowCursor2d,
			AssetLoadBlockAppReady,
			BundlePlaceholder::Sprite("spaceship_pack/planet_6.png".into()),
			Transform::from_translation(Vec3::new(200., 0., 0.)),
		))
		.id();

	// agent
	commands
		.spawn((
			// Transform::default(),
			BundlePlaceholder::Sprite("spaceship_pack/ship_2.png".into()),
			AssetLoadBlockAppReady,
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_to(500.).with_target(target),
		))
		.with_children(|parent| {
			// behavior
			parent.spawn((
				Name::new("Seek"),
				InsertOnTrigger::<AppReady, Running>::default(),
				TargetAgent(parent.parent_entity()),
				Seek,
			));
		});
}
