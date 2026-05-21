//! # Hello ML
//!
//! Minimal sentence-similarity demo: once the [`Bert`] asset finishes
//! loading, the agent's prompt ("please kill the baddies") is compared
//! to each child's [`Sentence`] and the winner is logged.
use beet::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AssetPlugin {
				meta_check: bevy::asset::AssetMetaCheck::Never,
				..default()
			},
			AsyncPlugin,
			ActionPlugin,
			BeetMlPlugins,
		))
		.add_systems(Startup, setup)
		.add_systems(Update, choose_when_loaded)
		.run();
}

/// Marker so the chooser system runs at most once.
#[derive(Component)]
struct Pending;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	let bert = asset_server.load::<Bert>("ml/default-bert.ron");
	commands.spawn((
		Name::new("Hello ML"),
		Pending,
		Sentence::new("please kill the baddies"),
		NearestSentence::new(bert),
		children![
			(Name::new("Heal Behavior"), Sentence::new("heal")),
			(Name::new("Attack Behavior"), Sentence::new("attack")),
		],
	));
}

fn choose_when_loaded(
	mut commands: Commands,
	mut berts: ResMut<Assets<Bert>>,
	sentences: Query<&Sentence>,
	names: Query<&Name>,
	mut pending: Query<
		(Entity, &Sentence, &NearestSentence, &Children),
		With<Pending>,
	>,
) -> Result {
	for (entity, prompt, near, children) in pending.iter_mut() {
		let Some(bert) = berts.get_mut(&near.bert) else {
			continue;
		};
		let chosen = bert.closest_sentence_entity(
			prompt.0.clone(),
			children.iter(),
			&sentences,
		)?;
		let name = names
			.get(chosen)
			.map(|n| n.to_string())
			.unwrap_or_else(|_| format!("{chosen}"));
		bevy::log::info!("NearestSentence chose: {name}");
		commands.entity(entity).remove::<Pending>();
	}
	Ok(())
}
