use crate::beet::prelude::*;
use crate::prelude::*;
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
					AssetLoadBlockAppReady,
					RunOnAppReady::default(),
					TargetAgent(agent),
					bert_handle,
					SentenceFlow::default(),
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
