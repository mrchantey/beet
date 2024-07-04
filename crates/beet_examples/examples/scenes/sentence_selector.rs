use beet::prelude::*;
// use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn sentence_selector(mut commands: Commands) {
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
							SentenceScorer::new(bert_handle),
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
