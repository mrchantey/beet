use super::*;
use beet::prelude::*;
use crate::prelude::*;
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
		} = load_foxie();

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
					Name::new("Animation Behavior"),
					Running,
					SequenceSelector,
					Repeat,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						TargetAgent(agent),
						SetAgentOnRun(Velocity::default()),
						PlayAnimation::new(idle_index)
							.with_transition_duration(transition_duration),
						idle_clip,
						InsertOnAnimationEnd::new(
							idle_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Seek"),
						TargetAgent(agent),
						Seek,
						PlayAnimation::new(walk_index)
							.repeat_forever()
							.with_transition_duration(transition_duration),
						SucceedOnArrive::new(6.),
					));
				});
		});
}
