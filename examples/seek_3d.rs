use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

fn main() {
	let mut app = App::new();

	app /*-*/
		.add_plugins(ExamplePlugin3d::default())
		.add_plugins(DefaultBeetPlugins::default())
		.add_plugins(BeetDebugPluginStdout)
		.add_systems(Startup, setup)
		.init_resource::<BeetDebugConfig>()
		.run();
}


fn setup(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) {
	// camera
	commands.spawn((
		CameraDistance {
			width: 80.,
			offset: Vec3::new(0., 20., 40.),
		},
		Camera3dBundle::default(),
	));

	// cheese
	let target = commands
		.spawn((FollowCursor3d, SceneBundle {
			scene: asset_server.load("kaykit/cheese.glb#Scene0"),
			transform: Transform::from_xyz(20., 0., 40.)
				.with_scale(Vec3::splat(3.)),
			..default()
		}))
		.id();

	let mut graph = AnimationGraph::new();

	let idle_anim_clip = asset_server.load("Fox.glb#Animation0");
	let idle_anim_index =
		graph.add_clip(idle_anim_clip.clone(), 1.0, graph.root);
	let walk_anim_clip = asset_server.load("Fox.glb#Animation1");
	let walk_anim_index =
		graph.add_clip(walk_anim_clip.clone(), 1.0, graph.root);

	let transition_duration = Duration::from_secs_f32(0.5);

	commands
		.spawn((
			SceneBundle {
				scene: asset_server.load("Fox.glb#Scene0"),
				transform: Transform::from_scale(Vec3::splat(0.1)),
				..default()
			},
			graphs.add(graph),
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
						PlayAnimation::new(idle_anim_index)
							.with_transition_duration(transition_duration),
						idle_anim_clip,
						InsertOnAnimationEnd::new(
							idle_anim_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						Name::new("Seek"),
						TargetAgent(agent),
						Seek,
						PlayAnimation::new(walk_anim_index)
							.repeat_forever()
							.with_transition_duration(transition_duration),
						SucceedOnArrive::new(6.),
					));
				});
		});
}
