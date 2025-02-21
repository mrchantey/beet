//! # Hello ML
//! A popular 'hello world' for machine learning in games is sentence similarity,
//! where models rank the similarity of sentences.
//! This example uses a locally run *small* language model to
//! select the child behavior with the most similar sentence.
//!
//! So if we ask the Bert to run the child with the nearest sentence
//! to "please kill the baddies", which action do you think it will take?
//!
//! Note that the [Name] components are for debugging only, its the
//! [Sentence] components that are used for the actual behavior selection.
use beet::prelude::*;
use bevy::prelude::*;

// #[rustfmt::skip]
pub fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			BeetFlowPlugin::default(),
			BeetDebugPlugin::default(),
			RunOnAssetReadyPlugin::<Bert>::default(),
			BertPlugin::default(),
		))
		.add_systems(Startup, (hello_ml,))
		.run();
}

#[rustfmt::skip]
fn hello_ml(mut commands: Commands, asset_server: Res<AssetServer>) {
	let handle = asset_server.load("ml/default-bert.ron");
	commands
		.spawn((
			Name::new("Hello ML"),
			HandleWrapper(handle.clone()),
			RunOnAssetReady::<Bert>::new(handle),
			NearestSentence::new(),
			Sentence::new("please kill the baddies"),
		))
		.with_child((
			Name::new("Heal Behavior"), 
			Sentence::new("heal")
		))
		.with_child((
			Name::new("Attack Behavior"), 
			Sentence::new("attack")
		));
}
