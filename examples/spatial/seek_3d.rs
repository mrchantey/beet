//! A combined example demonstrating a behavior 
//! that involves animation and steering.
use beet::examples::scenes;
use beet::prelude::*;
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
		CameraDistance {
			width: 80.,
			offset: Vec3::new(0., 20., 40.),
		},
	));

	let target = commands
		.spawn((
			Name::new("Cheese"),
			FollowCursor3d::default(),
			Transform::from_xyz(20., 0., 40.).with_scale(Vec3::splat(3.)),
			SceneRoot(asset_server.load("kaykit/cheese.glb#Scene0")),
		))
		.id();

	let Foxie {
		graph_handle,
		idle_index,
		idle_clip,
		walk_index,
		walk_clip: _,
	} = Foxie::new(&asset_server, &mut anim_graphs);

	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
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
			SteerTarget::Entity(target),
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
					Remove::<OnRun, Velocity>::new_with_target(
						TargetEntity::Origin,
					),
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
					Name::new("Seek"),
					Seek::default(),
					Insert::<OnRun, _>::new_with_target(
						Velocity::default(),
						TargetEntity::Origin,
					),
					PlayAnimation::new(walk_index)
						.repeat_forever()
						.with_transition_duration(transition_duration),
					EndOnArrive::new(6.),
				));
		});

	// 	parent.spawn((
	// 	));
	// });
}
