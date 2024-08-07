use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

pub type TriggerOnRun<T> = TriggerOnTrigger<OnRun, T>;

pub type TriggerOnTrigger<Event, Params, TriggerBundle = ()> =
	TriggerMappedOnTrigger<DefaultMapFunc<Event, Params, TriggerBundle>>;

pub type TriggerMappedOnTrigger<M> = OnTrigger<TriggerHandler<M>>;

#[derive(Reflect)]
pub struct TriggerHandler<T: MapFunc>(#[reflect(ignore)] PhantomData<T>);


impl<M: MapFunc> OnTriggerHandler for TriggerHandler<M>
where
	M::Out: Event + Clone,
{
	type Event = M::Event;
	type TriggerBundle = M::TriggerBundle;
	type Params = M::Params;
	fn handle(
		commands: &mut Commands,
		trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		(entity, on_trigger): (Entity, &OnTrigger<Self>),
	) {
		let out = M::map(trigger, (entity, &on_trigger.params));
		on_trigger.target.trigger(commands, entity, out);
	}
}

// see `end_on_run` for tests
