use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;




/// When [`OnUserMessage`] is triggered, run the [`OnRunAction`],
/// setting the [`OnUserMessage`] as the [`Sentence`].
#[action(run_with_user_sentence::<P>)]
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

fn run_with_user_sentence<P: RunPayload>(
	ev: Trigger<OnUserMessage>,
	mut commands: Commands,
	mut query: Query<(&RunWithUserSentence<P>, &mut Sentence)>,
) {
	for (run_with_user_sentence, mut sentence) in query.iter_mut() {
		sentence.0 = (**ev).clone().into();
		commands.trigger(run_with_user_sentence.trigger.clone());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
	}
}
