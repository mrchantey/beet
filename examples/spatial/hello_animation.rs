use beet::examples::scenes;
use beet::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				scenes::ui_terminal,
				scenes::lighting_3d,
				scenes::ground_3d,
				setup,
			),
		)
		.run();
}

fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut anim_graphs: ResMut<Assets<AnimationGraph>>,
) {
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		Transform::from_xyz(10.0, 10.0, 15.0)
			.looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
	));

	let Foxie {
		graph_handle,
		idle_index,
		idle_clip,
		walk_index,
		walk_clip,
	} = Foxie::new(&asset_server, &mut anim_graphs);


	let transition_duration = Duration::from_secs_f32(0.5);

	// a scene root will spawn the scene as a child, so we need the sequence
	// to be nested so it doesnt try to run the scene as a behavior.
	commands
		.spawn((
			Name::new("Foxie"),
			Transform::from_scale(Vec3::splat(0.1)),
			SceneRoot(asset_server.load("misc/fox.glb#Scene0")),
			graph_handle,
			// AnimationTransitions::default(),
		))
		.with_children(|parent| {
			parent
				.spawn((
					Name::new("Behavior"),
					RunOnAnimationReady::default(),
					Sequence::default(),
					Repeat::default(),
				))
				.with_child((
					Name::new("Idle"),
					PlayAnimation::new(idle_index)
						.with_transition_duration(transition_duration),
					ReturnOnAnimationEnd::new(
						idle_clip,
						idle_index,
						RunResult::Success,
					)
					.with_transition_duration(transition_duration),
				))
				.with_child((
					Name::new("Walking"),
					PlayAnimation::new(walk_index)
						.repeat(RepeatAnimation::Count(8))
						.with_transition_duration(transition_duration),
					ReturnOnAnimationEnd::new(
						walk_clip,
						walk_index,
						RunResult::Success,
					)
					.with_transition_duration(transition_duration),
				));
		});
}
