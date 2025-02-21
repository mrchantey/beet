//! # Hello ML
//! A popular 'hello world' for machine learning in games is sentence similarity,
//! where models rank the similarity of sentences.
//! This example uses a locally run *small* language model to select the child behavior with the most similar sentence.
use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins((
			running_beet_example_plugin, 
			plugin_ml
		))
		.init_resource::<DebugOnRun>()
		.add_systems(
			Startup,
			(
				scenes::camera_2d,
				scenes::ui_terminal_input,
				hello_ml,
			),
		)
		.run();
}


fn hello_ml(mut commands: Commands, asset_server: Res<AssetServer>) {
	let handle = asset_server.load("ml/default-bert.ron");
	commands.spawn((
		Name::new("Hello ML"),
		RunOnAssetReady::<Bert>::new(handle),
		SentenceBundle::with_initial("please kill the baddies"),
	));
	// .with_children(|parent| {
	// 	parent.spawn((Name::new("Heal Behavior"), Sentence::new("heal")));
	// 	parent
	// 		.spawn((Name::new("Attack Behavior"), Sentence::new("attack")));
	// });
}
