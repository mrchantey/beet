//! # Hello ML
//! A popular 'hello world' for machine learning in games is sentence similarity,
//! where models rank the similarity of sentences.
//! This example uses a locally run *small* language model to select the child behavior with the most similar sentence.
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			ExamplePlugin3d::default(),
			DefaultBeetPlugins,
			BeetDebugPluginStdout,
			BertPlugin::default(),
			ActionPlugin::<InsertOnAssetEvent<RunResult, Bert>>::default(),
		))
		.init_resource::<BeetDebugConfig>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	let bert_handle: Handle<Bert> = asset_server.load("default-bert.ron");

	commands
		.spawn((Name::new("Agent"), Sentence::new("please kill the baddies")))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			parent
				.spawn((Running, SequenceSelector))
				.with_children(|parent| {
					parent.spawn((
						Name::new("Await Bert Loaded"),
						InsertOnAssetEvent::loaded(
							RunResult::Success,
							&bert_handle,
						),
					));
					parent
						.spawn((
							Name::new("Sentence Selector"),
							TargetAgent(agent),
							bert_handle,
							SentenceScorer::default(),
							ScoreSelector {
								consume_scores: true,
							},
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
		});
}
/*
STDOUT:

Started: Await Bert Loaded
Started: Sentence Selector
Started: Attack Behavior

*/
