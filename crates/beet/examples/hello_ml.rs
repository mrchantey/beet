use beet::prelude::*;
use bevy::prelude::*;
#[path = "common/example_plugin_3d.rs"]
mod example_plugin_3d;
use example_plugin_3d::ExamplePlugin3d;

fn main() {
	App::new()
		.add_plugins((
			ExamplePlugin3d,
			DefaultBeetPlugins,
			BeetDebugPlugin::default(),
			MlPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn((Name::new("Agent"), Sentence::new("please kill the baddies")))
		.with_children(|parent| {
			let id = parent.parent_entity();
			parent
				.spawn((
					Name::new("Agent Behavior"),
					TargetAgent(id),
					SentenceScorer::new(asset_server.load("default-bert.ron")),
					ScoreSelector {
						consume_scores: true,
					},
					Running,
				))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Heal Behavior"),
						Sentence::new("heal"),
					));
					parent.spawn((
						Name::new("Attack Behavior"),
						Sentence::new("attack"),
					));
				});
		});
}
