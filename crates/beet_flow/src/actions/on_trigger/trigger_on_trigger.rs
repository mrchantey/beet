use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Trigger [`OnRun`] for the [`TriggerOnTrigger::target`]
/// whenever [`OnRunResult`] is triggered on one of the [`TriggerOnTrigger::sources`].
pub type RunOnRunResult = TriggerOnTrigger<OnRunResult, OnRun>;


pub type TriggerOnRun<T> = TriggerOnTrigger<OnRun, T>;

/// Trigger `EventOut` when `EventIn` is triggered.
/// Optionally accepts a `EventInBundle` for the `EventIn` trigger.
pub type TriggerOnTrigger<EventIn, EventOut, EventInBundle = ()> =
	TriggerMappedOnTrigger<DefaultMapFunc<EventIn, EventOut, EventInBundle>>;

pub type TriggerMappedOnTrigger<M> = OnTrigger<TriggerOnTriggerHandler<M>>;

#[derive(Reflect)]
pub struct TriggerOnTriggerHandler<T: OnTriggerMapFunc>(
	#[reflect(ignore)] PhantomData<T>,
);


impl<M: OnTriggerMapFunc> OnTriggerHandler for TriggerOnTriggerHandler<M>
where
	M::Out: Event + Clone,
{
	type TriggerEvent = M::Event;
	type TriggerBundle = M::TriggerBundle;
	type Params = M::Params;
	fn handle(
		commands: &mut Commands,
		trigger: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		(entity, on_trigger): (Entity, &OnTrigger<Self>),
	) {
		let out = M::map(trigger, (entity, &on_trigger.params));
		on_trigger.target.trigger(commands, entity, out);
	}
}

// see `end_on_run` for tests
