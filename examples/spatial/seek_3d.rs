use std::time::Duration;
use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

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

	// camera
	commands.spawn((
		Name::new("Camera"),
		Camera3d::default(),
		CameraDistance {
			width: 80.,
			offset: Vec3::new(0., 20., 40.),
		},
	));

	// cheese
	// let target = commands
	// 	.spawn((
	// 		FollowCursor3d::default(),
	// 		Transform::from_xyz(20., 0., 40.).with_scale(Vec3::splat(3.)),
	// 		SceneRoot(asset_server.load("kaykit/cheese.glb#Scene0")),
	// 	))
	// 	.id();

	let Foxie {
		graph_handle,
		idle_index,
		idle_clip,
		walk_index,
		walk_clip,
	} = Foxie::new(&asset_server, &mut anim_graphs);

	let transition_duration = Duration::from_secs_f32(0.5);

	commands.spawn((
		Name::new("Foxie"),
		Transform::from_scale(Vec3::splat(0.1)),
		SceneRoot(asset_server.load("misc/fox.glb#Scene0")),
		graph_handle,
		AnimationTransitions::new(),
		RotateToVelocity3d::default(),
		ForceBundle::default(),
		SteerBundle {
			max_force: MaxForce(0.05),
			..default()
		}
		.scaled_dist(10.),
		// SteerTarget::Entity(target),
		// RunOnSpawn::default(),
		Sequence::default(),
		Repeat::default(),
	));
	// .with_children(|parent| {
	// 	parent.spawn((
	// 		Name::new("Idle"),
	// 		// Remove::<OnRun, Velocity>::new_with_target(agent),
	// 		PlayAnimation::new(idle_index)
	// 			.with_transition_duration(transition_duration),
	// 		TriggerOnAnimationEnd::new(
	// 			idle_clip,
	// 			idle_index,
	// 			OnResultAction::local(RunResult::Success),
	// 		)
	// 		.with_transition_duration(transition_duration),
	// 	));
	// 	parent.spawn((
	// 		Name::new("Seek"),
	// 		Seek::default(),
	// 		// InsertOnRun::<Velocity>::new_with_target(agent),
	// 		PlayAnimation::new(walk_index)
	// 			.repeat_forever()
	// 			.with_transition_duration(transition_duration),
	// 		EndOnArrive::new(6.),
	// 	));
	// });
}
