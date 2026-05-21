use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_spatial::prelude::*;
use std::marker::PhantomData;

/// Action: picks the entity with the [`Sentence`] most similar to the
/// agent's prompt and sets it as the agent's [`SteerTarget`].
///
/// The generic parameter `F` filters the candidate pool to entities
/// matching `With<F>`.
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
#[require(Action<(), Outcome> = Action::<(), Outcome>::new_system(sentence_steer_target::<F>))]
pub struct SentenceSteerTarget<F: Component> {
	/// Asset handle for the [`Bert`] encoder.
	pub bert: Handle<Bert>,
	/// Entity carrying the [`Sentence`] used as the search prompt. Most
	/// commonly the agent itself; the indirection lets the prompt live
	/// on a sibling entity.
	pub target_entity: TargetEntity,
	#[reflect(ignore)]
	_phantom: PhantomData<F>,
}

impl<F: Component> SentenceSteerTarget<F> {
	/// Create a [`SentenceSteerTarget`] from a [`Bert`] handle and the
	/// entity that carries the prompt [`Sentence`].
	pub fn new(bert: Handle<Bert>, target_entity: TargetEntity) -> Self {
		Self {
			bert,
			target_entity,
			_phantom: PhantomData,
		}
	}
}

impl<F: Component> Default for SentenceSteerTarget<F> {
	fn default() -> Self {
		Self {
			bert: Handle::default(),
			target_entity: TargetEntity::default(),
			_phantom: PhantomData,
		}
	}
}

fn sentence_steer_target<F: Component>(
	cx: In<ActionContext>,
	mut commands: Commands,
	query: Query<&SentenceSteerTarget<F>>,
	sentences: Query<&Sentence>,
	items: Query<Entity, (With<Sentence>, With<F>)>,
	mut berts: ResMut<Assets<Bert>>,
	agent_query: AgentQuery,
) -> Result<Outcome> {
	let action = cx.caller.id();
	let target_action = query.get(action)?;
	let target_entity = target_action.target_entity.get(action, &agent_query);
	let target_sentence = sentences.get(target_entity)?;
	// Asset is downloaded asynchronously by [`BertLoader`]; if the user
	// triggers this action before the load finishes, soft-fail so the
	// sequence stops without panicking the app.
	let Some(bert) = berts.get_mut(&target_action.bert) else {
		log::warn!("Bert asset not yet loaded, ignoring action call");
		return Ok(Outcome::FAIL);
	};
	let agent = agent_query.entity(action);

	let chosen = bert.closest_sentence_entity(
		target_sentence.0.clone(),
		items.iter().filter(|e| *e != target_entity).collect::<Vec<_>>(),
		&sentences,
	)?;
	commands.entity(agent).insert(SteerTarget::Entity(chosen));
	Ok(Outcome::PASS)
}
