#![cfg(feature = "beet_core")]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
/// Finds the [`Sentence`] with the highest similarity to the agent's, then set it as the agent's [`SteerTarget`].
/// The generic parameter is used to filter the entities to consider.
pub struct FindSentenceSteerTarget<
	T: 'static + Send + Sync + QueryFilter = With<Sentence>,
> {
	pub bert: Handle<Bert>,
	// / The value below which the agent will ignore the target.
	// pub threshold:f32,
	phantom: std::marker::PhantomData<T>,
}

impl<T: 'static + Send + Sync + QueryFilter> FindSentenceSteerTarget<T> {
	pub fn new(bert: Handle<Bert>) -> Self {
		Self {
			bert,
			phantom: std::marker::PhantomData,
		}
	}
}

fn find_sentence_steer_target<T: 'static + Send + Sync + QueryFilter>(
	mut commands: Commands,
	query: Query<(&TargetAgent, &FindSentenceSteerTarget<T>), With<Running>>,
	sentences: Query<&Sentence>,
	// TODO this should be query of Sentence, but we need
	// it to be similar to sentence_scorer
	items: Query<Entity, T>,
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
			let (entity, _, _score) = scores[0];
			commands.entity(agent.0).insert(SteerTarget::Entity(entity));
		}

		// let options =

		//VERY EXPENSIVE
		// let embeddings = bert.get_embeddings(options).unwrap();

		// log::info!("Running - {}", find_sentence_steer_target);
	}
}

impl<T: 'static + Send + Sync + QueryFilter> ActionMeta
	for FindSentenceSteerTarget<T>
{
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl<T: 'static + Send + Sync + QueryFilter> ActionSystems
	for FindSentenceSteerTarget<T>
{
	fn systems() -> SystemConfigs {
		find_sentence_steer_target::<T>.in_set(TickSet)
	}
}


#[cfg(test)]
mod test {
	// use crate::ml_module::ml_plugin::MlPlugin;
	use crate::prelude::*;
	use anyhow::Result;
	use beet_core::steer::SteerTarget;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
		commands
			.spawn(Sentence::new("destroy"))
			.with_children(|parent| {
				let id = parent.parent_entity();
				parent
					.spawn((
						TargetAgent(id),
						FindSentenceSteerTarget::<With<Sentence>>::new(
							asset_server.load("default-bert.ron"),
						),
						Running,
					))
					.with_children(|parent| {
						parent.spawn(Sentence::new("heal"));
						parent.spawn(Sentence::new("kill"));
					});
			});
	}


	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			MlPlugin::default(),
			LifecyclePlugin,
		))
		.add_systems(Startup, setup)
		.finish();

		let entity = loop {
			app.update();
			let action = app
				.world_mut()
				.query::<&FindSentenceSteerTarget>()
				.iter(app.world())
				.next()
				.unwrap();

			if app
				.world()
				.get_resource::<Assets<Bert>>()
				.unwrap()
				.get(&action.bert)
				.is_some()
			{
				break app
					.world_mut()
					.query_filtered::<Entity, (Without<Parent>, With<Sentence>)>(
					)
					.iter(app.world())
					.next()
					.unwrap();
			}
			std::thread::sleep(std::time::Duration::from_millis(1));
		};

		let tree = EntityTree::new_with_world(entity, app.world());
		let kill = tree.children[0].children[1].value;

		let target = app.world().entity(entity).get::<SteerTarget>();
		expect(target).to_be(Some(&SteerTarget::Entity(kill)))?;

		Ok(())
	}
}
