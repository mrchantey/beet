use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;

fn main() {
	// pretty_env_logger::try_init().ok();

	App::new()
		.add_plugins((DefaultPlugins, DefaultBeetPlugins, MlPlugin::default()))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn(Sentence::new("destroy"))
		.with_children(|parent| {
			let id = parent.parent_entity();
			parent
				.spawn((
					LogOnRun::new("Selecting.."),
					TargetAgent(id),
					SentenceScorer::new(asset_server.load("default-bert.ron")),
					ScoreSelector {
						consume_scores: true,
					},
					Running,
				))
				.with_children(|parent| {
					parent.spawn((
						LogOnRun::new("Healing"),
						Sentence::new("heal"),
					));
					parent.spawn((
						LogOnRun::new("Killing"),
						Sentence::new("kill"),
					));
				});
		});
}
