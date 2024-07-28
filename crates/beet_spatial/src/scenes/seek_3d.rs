use crate::prelude::*;
use beet_examples::prelude::*;
use beet_flow::prelude::*;
use beetmash::core::scenes::Foxie;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::time::Duration;


pub fn seek_3d(mut commands: Commands) {
	// camera
	commands.spawn((BundlePlaceholder::Camera3d, CameraDistance {
		width: 80.,
		offset: Vec3::new(0., 20., 40.),
	}));

	// cheese
	let target = commands
		.spawn((
			FollowCursor3d,
			Transform::from_xyz(20., 0., 40.).with_scale(Vec3::splat(3.)),
			BundlePlaceholder::Scene("kaykit/cheese.glb#Scene0".into()),
		))
		.id();

	let Foxie {
		graph,
		idle_clip,
		idle_index,
		walk_index,
		..
	} = Foxie::default();

	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
			Transform::from_scale(Vec3::splat(0.1)),
			BundlePlaceholder::Scene("Fox.glb#Scene0".into()),
			graph,
			AnimationTransitions::new(),
			RotateToVelocity3d::default(),
			ForceBundle::default(),
			SteerBundle {
				max_force: MaxForce(0.05),
				..default()
			}
			.scaled_to(10.)
			.with_target(target),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Seek Behavior"),
					RunOnSpawn,
					SequenceFlow,
					Repeat::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						RemoveOnTrigger::<OnRun, Velocity>::default()
							.with_target(agent),
						ContinueRun::default(),
						TargetAgent(agent),
						PlayAnimation::new(idle_index)
							.with_transition_duration(transition_duration),
						idle_clip,
						TriggerOnAnimationEnd::new(
							idle_index,
							OnRunResult::success(),
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Seek"),
						ContinueRun::default(),
						TargetAgent(agent),
						Seek,
						InsertOnTrigger::<OnRun, Velocity>::default()
							.with_target(agent),
						PlayAnimation::new(walk_index)
							.repeat_forever()
							.with_transition_duration(transition_duration),
						EndOnArrive::new(6.),
					));
				});
		});
}
