use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;


pub fn animation_demo(mut commands: Commands) {
	// camera
	commands.spawn((
		BundlePlaceholder::Camera3d,
		Transform::from_xyz(10.0, 10.0, 15.0)
			.looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
	));

	let mut graph = AnimationGraphPlaceholder::default();

	let anim1_clip =
		AssetPlaceholder::<AnimationClip>::new("Fox.glb#Animation0");
	let anim1_index = graph.add_clip(anim1_clip.clone(), 1.0, graph.root);
	let anim2_clip =
		AssetPlaceholder::<AnimationClip>::new("Fox.glb#Animation1");
	let anim2_index = graph.add_clip(anim2_clip.clone(), 1.0, graph.root);

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
					Running,
					SequenceSelector,
					Repeat,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Idle"),
						TargetAgent(agent),
						PlayAnimation::new(anim1_index)
							.repeat(RepeatAnimation::Count(1))
							.with_transition_duration(transition_duration),
						anim1_clip,
						InsertOnAnimationEnd::new(
							anim1_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Walking"),
						TargetAgent(agent),
						PlayAnimation::new(anim2_index)
							.repeat(RepeatAnimation::Count(4))
							.with_transition_duration(transition_duration),
						anim2_clip,
						InsertOnAnimationEnd::new(
							anim2_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
				});
		});
}
