use std::marker::PhantomData;

use crate::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use bevy::prelude::*;

/// Finds the [`Sentence`] with the highest similarity to the agent's,
/// then set it as the agent's [`SteerTarget`].
/// The generic parameter is used to [`With`] filter the entities to consider.
#[action(sentence_steer_target::<F>)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
// TODO OnRun<Sentence>
// RunResult<SteerTarget?>

pub struct SentenceSteerTarget<F: Component> {
	pub target_entity: TargetEntity,
	// / The value below which the agent will ignore the target.
	// pub threshold:f32,
	// #[serde(bound = "")]
	_phantom: PhantomData<F>,
}

impl<F: Component> SentenceSteerTarget<F> {
	/// Create a new [`SentenceSteerTarget`] with the given [`TargetEntity`].
	pub fn new(target_entity: TargetEntity) -> Self {
		Self {
			target_entity,
			_phantom: PhantomData,
		}
	}
}

impl<F: Component> Default for SentenceSteerTarget<F> {
	fn default() -> Self {
		Self {
			target_entity: Default::default(),
			_phantom: PhantomData,
		}
	}
}

fn sentence_steer_target<F: Component>(
	ev: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<(&HandleWrapper<Bert>, &SentenceSteerTarget<F>)>,
	sentences: Query<&Sentence>,
	// TODO this should be query of Sentence, but we need
	// it to be similar to sentence_scorer
	items: Query<Entity, (With<Sentence>, With<F>)>,
	mut berts: ResMut<Assets<Bert>>,
) {
	let (handle, sentence_steer_target) = query
		.get(ev.action())
		.expect(&expect_action::to_have_action(&ev));

	let target_entity = sentence_steer_target.target_entity.get_target(&*ev);

	let target_sentence = sentences
		.get(target_entity)
		.expect(&expect_action::to_have_other(&ev));

	let bert = berts
		.get_mut(handle)
		.expect(&expect_action::to_have_asset(&ev));

	match bert.closest_sentence_entity(
		target_sentence.0.clone(),
		items
			.into_iter()
			.filter(|e| *e != target_entity)
			.collect::<Vec<_>>(),
		&sentences,
	) {
		Ok(entity) => {
			commands
				.entity(ev.origin())
				.insert(SteerTarget::Entity(entity));
		}
		Err(e) => log::error!("SentenceFlow: {}", e),
	}
}

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
// 			Language::default(),
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
