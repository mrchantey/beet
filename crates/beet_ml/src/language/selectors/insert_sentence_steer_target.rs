#![cfg(feature = "beet_spatial")]
use crate::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

// #[serde(bound = "")]
/// Finds the [`Sentence`] with the highest similarity to the agent's, then set it as the agent's [`SteerTarget`].
/// The generic parameter is used to [`With`] filter the entities to consider.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(insert_sentence_steer_target::<T>)]
pub struct InsertSentenceSteerTarget<T: GenericActionComponent = Sentence> {
	// / The value below which the agent will ignore the target.
	// pub threshold:f32,
	#[reflect(ignore)]
	phantom: PhantomData<T>,
}

impl<T: GenericActionComponent> Default for InsertSentenceSteerTarget<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

fn insert_sentence_steer_target<T: GenericActionComponent>(
	trigger: Trigger<OnInsert, Sentence>,
	mut commands: Commands,
	query: Query<(
		&TargetAgent,
		Option<&Sentence>,
		&Handle<Bert>,
		&InsertSentenceSteerTarget<T>,
	)>,
	sentences: Query<&Sentence>,
	// TODO this should be query of Sentence, but we need
	// it to be similar to sentence_scorer
	items: Query<Entity, With<T>>,
	mut berts: ResMut<Assets<Bert>>,
) {
	let (agent, target_sentence, handle, _) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let Some(bert) = berts.get_mut(handle) else {
		log::warn!("{}", expect_asset::NOT_READY);
		return;
	};
	let Some(target_sentence) = target_sentence else {
		log::warn!("{}", "sentence not set yet.. should this be allowed?");
		return;
	};
	match bert.closest_sentence_entity(
		target_sentence.0.clone(),
		items
			.into_iter()
			.filter(|e| *e != trigger.entity())
			.collect::<Vec<_>>(),
		&sentences,
	) {
		Ok(entity) => {
			commands.entity(agent.0).insert(SteerTarget::Entity(entity));
		}
		Err(e) => log::error!("SentenceFlow: {}", e),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_flow::prelude::*;
	use beet_spatial::steer::SteerTarget;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			BertPlugin::default(),
			LifecyclePlugin,
		))
		.finish();

		block_on_asset_load::<Bert>(&mut app, "default-bert.ron");

		let world = app.world_mut();

		let handle = world
			.resource_mut::<AssetServer>()
			.load::<Bert>("default-bert.ron");


		let agent = world.spawn_empty().id();

		let heal = world.spawn(Sentence::new("heal")).id();
		let kill = world.spawn(Sentence::new("kill")).id();

		let behavior = world
			.spawn((
				TargetAgent(agent),
				InsertSentenceSteerTarget::<Sentence>::default(),
				handle,
			))
			.id();
		world.flush();
		world.entity_mut(behavior).insert(Sentence::new("destroy"));
		world.flush();

		let target = world.entity(agent).get::<SteerTarget>();
		expect(target)
			.not()
			.to_be(Some(&SteerTarget::Entity(heal)))?;
		expect(target).to_be(Some(&SteerTarget::Entity(kill)))?;

		Ok(())
	}
}
