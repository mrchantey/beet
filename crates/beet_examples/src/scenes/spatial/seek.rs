use crate::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;


pub fn seek(mut commands: Commands) {
	let target = commands
		.spawn((
			Name::new("Target"),
			FollowCursor2d,
			AssetLoadBlockAppReady,
			BundlePlaceholder::Sprite("spaceship_pack/planet_6.png".into()),
			Transform::from_translation(Vec3::new(200., 0., 0.)),
		))
		.id();

	commands
		.spawn((
			Name::new("Agent"),
			BundlePlaceholder::Sprite("spaceship_pack/ship_2.png".into()),
			AssetLoadBlockAppReady,
			RotateToVelocity2d,
			ForceBundle::default(),
			SteerBundle::default().scaled_to(500.).with_target(target),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Seek"),
				RunOnAppReady::default(),
				ContinueRun::default(),
				TargetAgent(parent.parent_entity()),
				Seek,
			));
		});
}
