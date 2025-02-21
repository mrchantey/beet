// use crate::prelude::*;
use beet_flow::prelude::*;
// use beet_spatial::prelude::*;
use bevy::prelude::*;
// use std::marker::PhantomData;

// #[serde(bound = "")]
/// Finds the [`Sentence`] with the highest similarity to the agent's, then set it as the agent's [`SteerTarget`].
/// The generic parameter is used to [`With`] filter the entities to consider.
// #[action(insert_sentence_steer_target)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct InsertSentenceSteerTarget {
	// / The value below which the agent will ignore the target.
	// pub threshold:f32,
}


// TODO this is very awkward, we need a better pattern

// fn insert_sentence_steer_target(
// 	ev: Trigger<OnInsert, Sentence>,
// 	mut commands: Commands,
// 	query: Query<(
// 		Option<&Sentence>,
// 		&HandleWrapper<Bert>,
// 		&InsertSentenceSteerTarget,
// 	)>,
// 	sentences: Query<&Sentence>,
// 	// TODO this should be query of Sentence, but we need
// 	// it to be similar to sentence_scorer
// 	items: Query<Entity, With<Sentence>>,
// 	mut berts: ResMut<Assets<Bert>>,
// ) {
// 	let (target_sentence, handle, _) = query
// 		.get(ev.entity())
// 		.expect(&expect_action::to_have_action(&ev));

// 	let bert = berts
// 		.get_mut(handle)
// 		.expect(&expect_action::to_have_asset(&ev));
// 	let Some(target_sentence) = target_sentence else {
// 		log::warn!("{}", "sentence not set yet.. should this be allowed?");
// 		return;
// 	};
// 	match bert.closest_sentence_entity(
// 		target_sentence.0.clone(),
// 		items
// 			.into_iter()
// 			.filter(|e| *e != ev.entity())
// 			.collect::<Vec<_>>(),
// 		&sentences,
// 	) {
// 		Ok(entity) => {
// 			commands.entity(agent.0).insert(SteerTarget::Entity(entity));
// 		}
// 		Err(e) => log::error!("SentenceFlow: {}", e),
// 	}
// }

// #[cfg(test)]
// mod test {
// 	use crate::prelude::*;
// 	use beet_flow::prelude::*;
// 	use beet_spatial::steer::SteerTarget;
// 	use bevy::prelude::*;
// 	use sweet::prelude::*;

// 	#[test]
// 	fn works() {
// 		pretty_env_logger::try_init().ok();

// 		let mut app = App::new();
// 		app.add_plugins((
// 			MinimalPlugins,
// 			BeetFlowPlugin::default(),
// 			workspace_asset_plugin(),
// 			BertPlugin::default(),
// 		))
// 		.finish();

// 		let handle =
// 			block_on_asset_load::<Bert>(&mut app, "ml/default-bert.ron")
// 				.unwrap();

// 		let world = app.world_mut();

// 		let agent = world.spawn_empty().id();

// 		let heal = world.spawn(Sentence::new("heal")).id();
// 		let kill = world.spawn(Sentence::new("kill")).id();

// 		let behavior = world
// 			.spawn((
// 				TargetEntity(agent),
// 				InsertSentenceSteerTarget::<Sentence>::default(),
// 				HandleWrapper(handle),
// 			))
// 			.id();
// 		world.flush();
// 		world.entity_mut(behavior).insert(Sentence::new("destroy"));
// 		world.flush();

// 		let target = world.entity(agent).get::<SteerTarget>();
// 		expect(target).not().to_be(Some(&SteerTarget::Entity(heal)));
// 		expect(target).to_be(Some(&SteerTarget::Entity(kill)));
// 	}
// }
