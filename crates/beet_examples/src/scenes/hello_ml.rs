use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn hello_ml(mut commands: Commands) {
	commands
		.spawn((
			Name::new("Sentence Flow"),
			AssetRunOnReady::<Bert>::new("default-bert.ron"),
			SentenceBundle::with_initial("please kill the baddies"),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Heal Behavior"), 
				Sentence::new("heal")
			));
			parent.spawn((
				Name::new("Attack Behavior"), 
				Sentence::new("attack")
			));
		});
}
