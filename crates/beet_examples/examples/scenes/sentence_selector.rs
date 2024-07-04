use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn sentence_selector(mut commands: Commands) {
	let bert_handle = AssetPlaceholder::<Bert>::new("default-bert.ron");

	commands
		.spawn((Name::new("Agent"), Sentence::new("please kill the baddies")))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			parent
				.spawn((
					Name::new("Sentence Selector"),
					InsertOnTrigger::<AppReady, Running>::default(),
					SequenceSelector,
					TargetAgent(agent),
					AssetLoadBlockAppReady,
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
}
