use super::Sentence;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;

#[derive(Reflect)]
pub struct MapUserMessageToSentence;
impl MapFunc for MapUserMessageToSentence {
	type Event = OnUserMessage;
	type Params = ();
	type Out = Sentence;
	fn map(
		ev: &Trigger<Self::Event, Self::TriggerBundle>,
		_params: (Entity, &Self::Params),
	) -> Self::Out {
		Sentence::new(ev.event().0.clone())
	}
}

pub type SetSentenceOnUserInput =
	InsertMappedOnGlobalTrigger<MapUserMessageToSentence>;

pub type RunOnSentenceChange = TriggerOnTrigger<OnInsert, OnRun, Sentence>;
