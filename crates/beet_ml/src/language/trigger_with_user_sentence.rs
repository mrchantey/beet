use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;




/// When [`OnUserMessage`] is triggered, run the [`OnRunAction`],
/// setting the [`OnUserMessage`] as the [`Sentence`].
///
/// ## Warning
/// This requires the [`LanguagePlugin`] to be registered, and
/// that only registers the default payload, others must register
/// [`run_with_user_sentence`] manually.
#[derive(Debug, Component)]
#[require(Sentence)]
pub struct TriggerWithUserSentence<P = RequestEndResult> {
	/// The action to trigger.
	pub payload: P,
}

impl Default for TriggerWithUserSentence {
	fn default() -> Self { Self { payload: default() } }
}

impl<P> TriggerWithUserSentence<P> {
	/// Create a new [`RunWithUserSentence`] with the given [`OnRunAction`].
	pub fn new(payload: P) -> Self { Self { payload } }
}

pub fn trigger_with_user_sentence<P: IntoEntityEvent + Clone>(
	ev: On<UserMessage>,
	mut commands: Commands,
	mut query: Query<(Entity, &TriggerWithUserSentence<P>, &mut Sentence)>,
) {
	for (action, run_with_user_sentence, mut sentence) in query.iter_mut() {
		sentence.0 = (**ev).clone().into();
		commands
			.entity(action)
			.trigger_entity(run_with_user_sentence.payload.clone());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default())
			.add_observer(trigger_with_user_sentence::<RequestEndResult>);
		let world = app.world_mut();
		let on_run = observer_ext::observe_triggers::<Run>(world);

		let entity = world
			.spawn((TriggerWithUserSentence::default(), EndOnRun(SUCCESS)))
			.id();
		world.flush();

		world.flush_trigger(UserMessage::new("pizza"));

		on_run.len().xpect_eq(1);
		world
			.get::<Sentence>(entity)
			.xpect_eq(Some(&Sentence::new("pizza")));
	}
}
