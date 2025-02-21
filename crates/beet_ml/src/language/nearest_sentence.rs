use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// This component is for use with [`SentenceFlow`]. Add to either the agent or a child behavior.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Sentence(pub Cow<'static, str>);
impl Sentence {
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

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
	ev: Trigger<OnRun>,
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
		.get(ev.action)
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
			ev.trigger_next(&mut commands, entity);
		}
		Err(e) => log::error!("SentenceFlow: {}", e),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "we need to get sentence_flow back up and running, ie the req/res model like score"]
	fn works() {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			workspace_asset_plugin(),
			BertPlugin::default(),
			BeetFlowPlugin::default(),
		))
		.finish();
		let on_run = observe_trigger_names::<OnRun>(app.world_mut());

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
			.flush_trigger(OnRun::local());


		expect(&on_run).to_have_been_called_times(2);
		expect(&on_run).to_have_returned_nth_with(0, &"root".to_string());
		expect(&on_run).to_have_returned_nth_with(1, &"kill".to_string());
	}
}
