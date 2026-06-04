use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// When a [`UserMessage`] is triggered, copy its text onto this entity's
/// [`Sentence`] and call the entity's [`Action`].
///
/// Wires a terminal-style chat input to a behavior tree: as the user types
/// commands, each one drives a fresh call of the attached action with the
/// new prompt available to any [`Sentence`]-based action below it (eg
/// [`SentenceSteerTarget`](crate::prelude::SentenceSteerTarget)).
///
/// The [`Sentence`] is required so the prompt slot always exists.
#[derive(Debug, Default, Component)]
#[require(Sentence)]
pub struct TriggerWithUserSentence;

/// Observer system: writes [`UserMessage`] text into each matching
/// [`Sentence`] and queues an [`Action<(), Outcome>`] call.
pub fn trigger_with_user_sentence(
	ev: On<UserMessage>,
	mut commands: Commands,
	mut query: Query<(Entity, &mut Sentence), With<TriggerWithUserSentence>>,
) {
	for (action, mut sentence) in query.iter_mut() {
		sentence.0 = ev.event().0.clone().into();
		commands
			.entity(action)
			.call::<(), Outcome>((), OutHandler::default());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn writes_sentence_and_calls() {
		let mut world = AsyncPlugin::world();
		world.commands().add_observer(trigger_with_user_sentence);
		world.flush();

		let store = Store::<u32>::default();
		let counter = store.clone();
		let entity = world
			.spawn((
				TriggerWithUserSentence,
				Action::<(), Outcome>::new_pure(move |_| {
					counter.set(counter.get() + 1);
					Ok(Outcome::PASS)
				}),
			))
			.id();
		world.flush();

		world.commands().trigger(UserMessage::new("pizza"));
		world.flush();

		world
			.get::<Sentence>(entity)
			.unwrap()
			.0
			.as_ref()
			.xpect_eq("pizza");
		store.get().xpect_eq(1);
	}
}
