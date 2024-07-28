use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;





pub fn flock(mut commands: Commands) {
	commands.insert_resource(WrapAround::default());

	for _ in 0..300 {
		let position = Vec3::ZERO;
		// Vec3::random_in_sphere() * 500.,
		commands
			.spawn((
				BundlePlaceholder::Sprite("spaceship_pack/ship_2.png".into()),
				Transform::from_translation(position)
					.with_scale(Vec3::splat(0.5)),
				RotateToVelocity2d,
				ForceBundle::default(),
				SteerBundle::default().scaled_to(100.),
				VelocityScalar(Vec3::new(1., 1., 0.)),
				GroupSteerAgent,
			))
			.with_children(|agent| {
				// behavior
				agent.spawn((
					RunOnSpawn,
					ContinueRun::default(),
					TargetAgent(agent.parent_entity()),
					Separate::<GroupSteerAgent>::new(1.),
					Align::<GroupSteerAgent>::new(1.),
					Cohere::<GroupSteerAgent>::new(1.),
					Wander::new(0.1),
				));
			});
	}
}
