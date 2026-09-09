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
#[action]
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
pub fn SentenceSteerTarget<F>(
	/// Asset handle for the [`Bert`] encoder.
	#[field]
	bert: Handle<Bert>,
	/// Entity carrying the [`Sentence`] used as the search prompt. Most
	/// commonly the agent itself; the indirection lets the prompt live
	/// on a sibling entity.
	#[field]
	target_entity: TargetEntity,
	cx: In<ActionContext>,
	mut commands: Commands,
	sentences: Query<&Sentence>,
	items: Query<Entity, (With<Sentence>, With<F>)>,
	mut berts: ResMut<Assets<Bert>>,
	agent_query: AgentQuery,
) -> Result<Outcome>
where
	F: Component,
{
	let action = cx.caller.id();
	let target_entity = target_entity.get(action, &agent_query);
	let target_sentence = sentences.get(target_entity)?;
	// Asset is downloaded asynchronously by [`BertLoader`]; if the user
	// triggers this action before the load finishes, soft-fail so the
	// sequence stops without panicking the app.
	let Some(mut bert) = berts.get_mut(&bert.clone()) else {
		log::warn!("Bert asset not yet loaded, ignoring action call");
		return Ok(Outcome::FAIL);
	};
	let agent = agent_query.entity(action);

	let chosen = bert.closest_sentence_entity(
		target_sentence.0.clone(),
		items
			.iter()
			.filter(|e| *e != target_entity)
			.collect::<Vec<_>>(),
		&sentences,
	)?;
	commands.entity(agent).insert(SteerTarget::Entity(chosen));
	Ok(Outcome::PASS)
}

impl<F: Component> SentenceSteerTarget<F> {
	/// Steer toward the `With<F>` entity whose [`Sentence`] best matches the
	/// prompt on `target_entity`.
	pub fn new(bert: Handle<Bert>, target_entity: TargetEntity) -> Self {
		Self {
			bert,
			target_entity,
			_marker: PhantomData,
		}
	}
}
