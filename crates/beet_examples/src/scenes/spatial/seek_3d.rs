use crate::beet::prelude::*;
use crate::prelude::*;
use bevyhub::core::scenes::Foxie;
use bevyhub::prelude::*;
use bevy::prelude::*;
use std::time::Duration;


pub fn seek_3d(mut commands: Commands) {
	// camera
	commands.spawn((
		Name::new("Camera"),
		BundlePlaceholder::Camera3d,
		CameraDistance {
			width: 80.,
			offset: Vec3::new(0., 20., 40.),
		},
	));

	// cheese
	let target = commands
		.spawn((
			FollowCursor3d::default(),
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
			Name::new("Foxie"),
			Transform::from_scale(Vec3::splat(0.1)),
			BundlePlaceholder::Scene("misc/fox.glb#Scene0".into()),
			graph,
			AnimationTransitions::new(),
			RotateToVelocity3d::default(),
			ForceBundle::default(),
			SteerBundle {
				max_force: MaxForce(0.05),
				..default()
			}
			.scaled_dist(10.),
			SteerTarget::Entity(target),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Seek Behavior"),
					RunOnSpawn,
					SequenceFlow,
					RepeatFlow::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						RemoveOnRun::<Velocity>::new_with_target(agent),
						TargetEntity(agent),
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
						TargetEntity(agent),
						Seek::default(),
						InsertOnRun::<Velocity>::new_with_target(agent),
						PlayAnimation::new(walk_index)
							.repeat_forever()
							.with_transition_duration(transition_duration),
						EndOnArrive::new(6.),
					));
				});
		});
}
