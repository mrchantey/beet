use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Action: picks the child entity whose [`Sentence`] is closest to the
/// agent's [`Sentence`] (its current prompt) and returns that entity as
/// the action's output.
///
/// Rearchitected from the old beet_flow shape — instead of triggering the
/// nearest child (a fan-out side-effect), this action now *returns* the
/// chosen entity, letting the caller compose. Pair it with a downstream
/// action (eg via `chain`) to act on the chosen entity.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(Action<(), Entity> = Action::<(), Entity>::new_system(nearest_sentence_system))]
pub struct NearestSentence {
	/// Handle to the [`Bert`] asset used to compute embeddings.
	pub bert: Handle<Bert>,
}

impl NearestSentence {
	/// Create a [`NearestSentence`] using the given [`Bert`] handle.
	pub fn new(bert: Handle<Bert>) -> Self { Self { bert } }
}

fn nearest_sentence_system(
	cx: In<ActionContext>,
	mut berts: ResMut<Assets<Bert>>,
	agents: AgentQuery<&Sentence>,
	sentences: Query<&Sentence>,
	children: Query<&Children>,
	query: Query<&NearestSentence>,
) -> Result<Entity> {
	let action_entity = cx.caller.id();
	let near = query.get(action_entity)?;
	let mut bert = berts.get_mut(&near.bert).ok_or_else(|| {
		bevyhow!("Bert asset not loaded for entity {action_entity:?}")
	})?;
	let prompt = agents.get(action_entity)?;
	let candidates = children
		.get(action_entity)
		.map(|c| c.iter().collect::<Vec<_>>())
		.unwrap_or_default();
	if candidates.is_empty() {
		bevybail!("NearestSentence: no children to choose from");
	}
	bert.closest_sentence_entity(prompt.0.clone(), candidates, &sentences)
}
