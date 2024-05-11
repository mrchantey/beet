use beet::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use forky_bevy::prelude::close_on_esc;
use std::time::Duration;
#[path = "common/setup_scene_3d.rs"]
mod setup_scene_3d;

pub fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				fit_canvas_to_parent: true,
				..default()
			}),
			..default()
		}))
		.add_plugins(DefaultBeetPlugins)
		.add_plugins(beet::prelude::AnimationPlugin)
		.add_systems(Startup, (setup_scene_3d::setup_scene_3d, setup_fox))
		.add_systems(Update, close_on_esc)
		.run();
}


fn setup_fox(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) {
	// Build the animation graph
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
				transform: Transform::default(),
				..default()
			},
			graphs.add(graph),
			AnimationTransitions::new(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();
			parent
				.spawn((Running, Repeat, SequenceSelector))
				.with_children(|parent| {
					parent.spawn((
						LogOnRun::new("running 1"),
						TargetAgent(agent),
						PlayAnimation::new(anim1_index)
							.repeat(RepeatAnimation::Count(1))
							.with_transition_duration(transition_duration),
						RunTimer::default(),
						InsertOnAnimationEnd::new(
							anim1_clip,
							anim1_index,
							RunResult::Success,
						)
						.with_transition_duration(transition_duration),
					));
					parent.spawn((
						LogOnRun::new("running 2"),
						TargetAgent(agent),
						RunTimer::default(),
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
