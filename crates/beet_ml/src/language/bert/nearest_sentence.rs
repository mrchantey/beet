use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Runs the child with the [`Sentence`] that is most similar to that of the agent.
/// for use with [`ScoreFlow`]
#[action(nearest_sentence)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
// #[require(Sentence=||Sentence::new("placeholder"))]
pub struct NearestSentence;

impl NearestSentence {
	pub fn new() -> Self { Self {} }
}

fn nearest_sentence(
	ev: On<Run>,
	mut commands: Commands,
	mut berts: ResMut<Assets<Bert>>,
	sentences: Query<&Sentence>,
	// TODO double query, ie added running and added asset
	query: Query<(
		&NearestSentence,
		&Sentence,
		&HandleWrapper<Bert>,
		&Children,
	)>,
) {
	let (_scorer, target_sentence, handle, children) = query
		.get(ev.event_target())
		.expect(&expect_action::to_have_action(&ev));
	let bert = berts
		.get_mut(handle)
		.expect(&expect_action::to_have_asset(&ev));
	match bert.closest_sentence_entity(
		target_sentence.0.clone(),
		children.iter().map(|e| e.clone()),
		&sentences,
	) {
		Ok(entity) => {
			commands.entity(entity).trigger_payload(GetOutcome);
		}
		Err(e) => log::error!("SentenceFlow: {}", e),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "we need to get sentence_flow back up and running, ie the req/res model like score"]
	fn works() {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			workspace_asset_plugin(),
			LanguagePlugin::default(),
			BeetFlowPlugin::default(),
		))
		.finish();
		let on_run =
			observer_ext::observe_trigger_names::<Run>(app.world_mut());

		let handle =
			block_on_asset_load::<Bert>(&mut app, "ml/default-bert.ron")
				.unwrap();

		app.world_mut()
			.spawn((
				Name::new("root"),
				Sentence::new("destroy"),
				HandleWrapper(handle),
				NearestSentence::default(),
			))
			.with_children(|parent| {
				parent.spawn((Name::new("heal"), Sentence::new("heal")));
				parent.spawn((Name::new("kill"), Sentence::new("kill")));
			})
			.trigger_payload(GetOutcome)
			.flush();


		on_run.len().xpect_eq(2);
		on_run.get_index(0).xpect_eq(Some("root".to_string()));
		on_run.get_index(1).xpect_eq(Some("kill".to_string()));
	}
}
