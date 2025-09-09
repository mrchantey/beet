use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;




/// When [`OnUserMessage`] is triggered, run the [`OnRunAction`],
/// setting the [`OnUserMessage`] as the [`Sentence`].
///
/// ## Warning
/// This requires the [`LanguagePlugin`] to be registered, and
/// that only registers the default payload, others must register
/// [`run_with_user_sentence`] manually.
#[derive(Debug, Component)]
#[require(Sentence)]
pub struct RunWithUserSentence<P: RunPayload = ()> {
	/// The action to trigger.
	pub trigger: OnRunAction<P>,
}

impl Default for RunWithUserSentence<()> {
	fn default() -> Self {
		Self {
			trigger: OnRunAction::default(),
		}
	}
}

impl<P: RunPayload> RunWithUserSentence<P> {
	/// Create a new [`RunWithUserSentence`] with the given [`OnRunAction`].
	pub fn new(trigger: OnRunAction<P>) -> Self { Self { trigger } }
}

pub fn run_with_user_sentence<P: RunPayload>(
	ev: Trigger<OnUserMessage>,
	mut commands: Commands,
	mut query: Query<(Entity, &RunWithUserSentence<P>, &mut Sentence)>,
) {
	for (action, run_with_user_sentence, mut sentence) in query.iter_mut() {
		sentence.0 = (**ev).clone().into();
		commands
			.entity(action)
			.trigger(run_with_user_sentence.trigger.clone());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default())
			.add_observer(run_with_user_sentence::<()>);
		let world = app.world_mut();
		let on_run = observe_triggers::<OnRun>(world);

		let entity = world
			.spawn((
				RunWithUserSentence::default(),
				ReturnWith(RunResult::Success),
			))
			.id();
		world.flush();

		world.flush_trigger(OnUserMessage::new("pizza"));

		on_run.len().xpect_eq(1);
		world
			.get::<Sentence>(entity)
			.xpect_eq(Some(&Sentence::new("pizza")));
	}
}
