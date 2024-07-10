use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


pub trait OnTriggerHandler: 'static + Send + Sync + Sized {
	type Event: Event;
	/// The bundle used for the Trigger, ie Trigger<E,B>
	type TriggerBundle: Bundle = ();
	type Params: 'static + Send + Sync + Default + Reflect;
	fn handle(
		commands: &mut Commands,
		ev: &Trigger<Self::Event, Self::TriggerBundle>,
		query: (Entity, &OnTrigger<Self>),
	);
}

pub trait MapFunc: 'static + Send + Sync {
	type Event: Event;
	type TriggerBundle: Bundle = ();
	type Params: 'static + Send + Sync + Clone + Default + Reflect;
	type Out: Bundle;
	fn map(
		trigger: &Trigger<Self::Event, Self::TriggerBundle>,
		target: (Entity, &Self::Params),
	) -> Self::Out;
}
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct DefaultMapFunc<E, T, B>(#[reflect(ignore)] PhantomData<(E, T, B)>);
impl<E: Event, T: Bundle + Clone + Default + Reflect, B: Bundle> MapFunc
	for DefaultMapFunc<E, T, B>
{
	type Event = E;
	type TriggerBundle = B;
	type Params = T;
	type Out = T;
	fn map(_ev: &Trigger<E, B>, params: (Entity, &T)) -> T { params.1.clone() }
}
