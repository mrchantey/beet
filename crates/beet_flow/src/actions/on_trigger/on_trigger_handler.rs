use crate::prelude::*;
use bevy::prelude::*;

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
	/// Override the default [`ActionTarget::This`] source, ie [`ActionTarget::Global`]
	fn default_source() -> ActionTarget { default() }
	/// Override the default [`ActionTarget::This`] target, ie [`ActionTarget::Global`]
	fn default_target() -> ActionTarget { default() }
}
