use crate::prelude::*;
use beet_flow::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

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

pub type InsertSentenceOnUserInput =
	InsertMappedOnGlobalTrigger<MapUserMessageToSentence>;

pub type RunOnSentenceChange = TriggerOnTrigger<OnInsert, OnRun, Sentence>;

#[derive(Bundle, Default)]
pub struct SentenceBundle {
	pub flow: SentenceFlow,
	pub run_on_change: RunOnSentenceChange,
	pub set_on_input: InsertSentenceOnUserInput,
}
impl SentenceBundle {
	pub fn with_initial(sentence: impl Into<Cow<'static, str>>) -> impl Bundle {
		(Self::default(), Sentence::new(sentence))
	}
}
