use beet::prelude::*;
use beetmash::core::scenes::Foxie;
use beetmash::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;

pub fn hello_animation(mut commands: Commands) {
	// camera
	commands.spawn((
		BundlePlaceholder::Camera3d,
		Transform::from_xyz(10.0, 10.0, 15.0)
			.looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
	));

	let Foxie {
		graph,
		idle_clip,
		idle_index,
		walk_clip,
		walk_index,
	} = default();


	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
			Transform::from_scale(Vec3::splat(0.1)),
			BundlePlaceholder::Scene("Fox.glb#Scene0".into()),
			graph,
			AnimationTransitions::new(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((
					Name::new("Animation Behavior"),
					RunOnSpawn,
					SequenceFlow,
					Repeat::default(),
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						ContinueRun::default(),
						TargetAgent(agent),
						PlayAnimation::new(idle_index)
							.repeat(RepeatAnimation::Count(1))
							.with_transition_duration(transition_duration),
						idle_clip,
						TriggerOnAnimationEnd::new(
							idle_index,
							OnRunResult::success(),
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Walking"),
						ContinueRun::default(),
						TargetAgent(agent),
						PlayAnimation::new(walk_index)
							.repeat(RepeatAnimation::Count(4))
							.with_transition_duration(transition_duration),
						walk_clip,
						TriggerOnAnimationEnd::new(
							walk_index,
							OnRunResult::success(),
						)
						.with_transition_duration(transition_duration),
					));
				});
		});
}
