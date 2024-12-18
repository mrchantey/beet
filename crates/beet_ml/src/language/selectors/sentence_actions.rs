use crate::prelude::*;
use beet_flow::prelude::*;
use bevyhub::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

#[derive(Reflect)]
pub struct MapUserMessageToSentence;
impl OnTriggerHandler for MapUserMessageToSentence {
	type TriggerEvent = OnUserMessage;

	fn default_source() -> ActionTarget { ActionTarget::Global }

	fn handle(
		commands: &mut Commands,
		ev: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		query: (Entity, &OnTrigger<Self>),
	) {
		let msg = ev.event().to_string();
		commands.entity(query.0).insert(Sentence::new(msg));
	}
}


pub type InsertSentenceOnUserInput = OnTrigger<MapUserMessageToSentence>;

pub type RunOnInsertSentence = TriggerOnTrigger<OnRun, OnInsert, Sentence>;

#[derive(Bundle, Default)]
pub struct SentenceBundle {
	pub flow: SentenceFlow,
	pub run_on_change: RunOnInsertSentence,
	pub set_on_input: InsertSentenceOnUserInput,
}
impl SentenceBundle {
	pub fn with_initial(sentence: impl Into<Cow<'static, str>>) -> impl Bundle {
		(Self::default(), Sentence::new(sentence))
	}
}
