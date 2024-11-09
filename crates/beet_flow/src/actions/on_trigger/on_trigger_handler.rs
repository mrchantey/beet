use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Trait for handling a trigger event
pub trait OnTriggerHandler: 'static + Send + Sync + Sized {
	/// The event used for the trigger, ie [`Trigger<TriggerEvent>`] 
	type TriggerEvent: Event;
	/// The bundle used for the Trigger, ie [`Trigger<TriggerEvent,TriggerBundle>`]
	type TriggerBundle: Bundle = ();
	/// Parameters used by the handler
	type Params: 'static + Send + Sync + Default + Reflect = ();
	fn handle(
		commands: &mut Commands,
		ev: &Trigger<Self::TriggerEvent, Self::TriggerBundle>,
		query: (Entity, &OnTrigger<Self>),
	);
}

/// Map some input value to an output value, commonly used as a
/// simpler abstraction level by implementers of [`OnTriggerHandler`]
pub trait OnTriggerMapFunc: 'static + Send + Sync {
	type Event: Event;
	type TriggerBundle: Bundle = ();
	type Params: 'static + Send + Sync + Clone + Default + Reflect;
	type Out: Bundle;
	fn map(
		trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		target: (Entity, &Self::Params),
	) -> Self::Out;
}

/// Simply clone and pass `Params`
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct DefaultMapFunc<E, T, B>(#[reflect(ignore)] PhantomData<(E, T, B)>);
impl<E: Event, T: Bundle + Clone + Default + Reflect, B: Bundle>
	OnTriggerMapFunc for DefaultMapFunc<E, T, B>
{
	type Event = E;
	type TriggerBundle = B;
	type Params = T;
	type Out = T;
	fn map(_ev: &Trigger<E, B>, params: (Entity, &T)) -> T { params.1.clone() }
}
