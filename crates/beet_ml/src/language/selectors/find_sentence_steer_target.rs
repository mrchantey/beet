#![cfg(feature = "beet_core")]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
// #[serde(bound = "")]
/// Finds the [`Sentence`] with the highest similarity to the agent's, then set it as the agent's [`SteerTarget`].
/// The generic parameter is used to [`With`] filter the entities to consider.
pub struct FindSentenceSteerTarget<T: GenericActionComponent = Sentence> {
	pub bert: Handle<Bert>,
	// / The value below which the agent will ignore the target.
	// pub threshold:f32,
	#[reflect(ignore)]
	phantom: std::marker::PhantomData<T>,
}

impl<T: GenericActionComponent> FindSentenceSteerTarget<T> {
	pub fn new(bert: Handle<Bert>) -> Self {
		Self {
			bert,
			phantom: std::marker::PhantomData,
		}
	}
}

fn find_sentence_steer_target<T: GenericActionComponent>(
	mut commands: Commands,
	query: Query<(&TargetAgent, &FindSentenceSteerTarget<T>), Added<Running>>,
	sentences: Query<&Sentence>,
	// TODO this should be query of Sentence, but we need
	// it to be similar to sentence_scorer
	items: Query<Entity, With<T>>,
	mut berts: ResMut<Assets<Bert>>,
) {
	for (agent, action) in query.iter() {
		let Some(bert) = berts.get_mut(&action.bert) else {
			continue;
		};

		let options = items.into_iter().collect::<Vec<_>>();

		//TODO: VERY EXPENSIVE
		if let Some(scores) = bert
			.score_sentences(agent.0, options, &sentences)
			.ok_or(|e| log::error!("{e}"))
			&& scores.len() > 0
		{
			let (target, _, _score) = scores[0];
			// log::info!("Setting target to {:?}", target);
			commands.entity(agent.0).insert(SteerTarget::Entity(target));
		}
	}
}

impl<T: GenericActionComponent> ActionMeta for FindSentenceSteerTarget<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<T: GenericActionComponent> ActionSystems for FindSentenceSteerTarget<T> {
	fn systems() -> SystemConfigs {
		find_sentence_steer_target::<T>.in_set(TickSet)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_core::steer::SteerTarget;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	fn setup(app: &mut App) -> Entity {
		let handle = app
			.world_mut()
			.resource_mut::<AssetServer>()
			.load::<Bert>("default-bert.ron");


		app.world_mut()
			.spawn(Sentence::new("destroy"))
			.with_children(|parent| {
				let id = parent.parent_entity();
				parent.spawn((
					TargetAgent(id),
					FindSentenceSteerTarget::<Sentence>::new(handle),
					Running,
				));
			})
			.id()
	}


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

		let entity = setup(&mut app);
		let _heal = app.world_mut().spawn(Sentence::new("heal")).id();
		let kill = app.world_mut().spawn(Sentence::new("kill")).id();

		app.update();

		let target = app.world().entity(entity).get::<SteerTarget>();
		expect(target).to_be(Some(&SteerTarget::Entity(kill)))?;

		Ok(())
	}
}
