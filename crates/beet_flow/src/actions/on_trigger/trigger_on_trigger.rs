use crate::prelude::*;
use bevy::prelude::*;
use bevy::scene::SceneInstanceReady;
use std::marker::PhantomData;


/// Trigger [`OnRun`] for the [`TriggerOnTrigger::target`]
/// whenever [`OnRunResult`] is triggered on one of the [`TriggerOnTrigger::sources`].
/// This is used by state machines and other paradigms that require
/// arbitary triggering of behaviors.
pub type RunOnRunResult = TriggerOnTrigger<OnRun, OnRunResult>;

/// Trigger [`OnRun`] for the [`TriggerOnTrigger::target`]
/// whenever [`SceneInstanceReady`] is triggered on one of the [`TriggerOnTrigger::sources`].
pub type RunOnSceneReady = TriggerOnTrigger<OnRun, SceneInstanceReady>;

pub type TriggerOnRun<T> = TriggerOnTrigger<T, OnRun>;

/// Trigger `EventOut` when `EventIn` is triggered.
/// Optionally accepts a `EventInBundle` for the `EventIn` trigger.
pub type TriggerOnTrigger<Params, TriggerEvent, TriggerBundle = ()> =
	OnTrigger<TriggerOnTriggerHandler<Params, TriggerEvent, TriggerBundle>>;

#[derive(Reflect)]
pub struct TriggerOnTriggerHandler<Params, TriggerEvent, TriggerBundle>(
	#[reflect(ignore)] PhantomData<(Params, TriggerEvent, TriggerBundle)>,
);


impl<
		Params: Default + Event + Reflect + Clone,
		TriggerEvent: Event,
		TriggerBundle: Bundle,
	> OnTriggerHandler
	for TriggerOnTriggerHandler<Params, TriggerEvent, TriggerBundle>
{
	type Params = Params;
	type TriggerEvent = TriggerEvent;
	type TriggerBundle = TriggerBundle;
	fn handle(
		commands: &mut Commands,
		_trigger: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		(entity, on_trigger): (Entity, &OnTrigger<Self>),
	) {
		on_trigger
			.target
			.trigger(commands, entity, on_trigger.params.clone());
	}
}

// see `end_on_run` for tests
