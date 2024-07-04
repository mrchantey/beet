use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;

pub fn main() {
	App::new()
		.add_plugins(ExamplePlugin3d::default())
		.add_plugins(DefaultBeetPlugins)
		.add_plugins(BeetDebugPluginStdout)
		.add_systems(Startup, (setup_camera, setup_fox))
		.init_resource::<BeetDebugConfig>()
		.run();
}


fn setup_camera(mut commands: Commands) {
	commands.spawn(Camera3dBundle {
		transform: Transform::from_xyz(10.0, 10.0, 15.0)
			.looking_at(Vec3::new(0.0, 2.0, 0.0), Vec3::Y),
		..default()
	});
}

fn setup_fox(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) {
	let mut graph = AnimationGraph::new();

	let anim1_clip = asset_server.load("Fox.glb#Animation0");
	let anim1_index = graph.add_clip(anim1_clip.clone(), 1.0, graph.root);
	let anim2_clip = asset_server.load("Fox.glb#Animation1");
	let anim2_index = graph.add_clip(anim2_clip.clone(), 1.0, graph.root);

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
						InsertOnAnimationEnd::new(
							anim1_clip,
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
						InsertOnAnimationEnd::new(
							anim2_clip,
							anim2_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
				});
		});
}
