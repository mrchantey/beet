use bevy::prelude::*;

/// Trait for handling a trigger event
pub trait OnChangeHandler: 'static + Send + Sync + Sized {
	/// The event used for the trigger, ie [`Trigger<TriggerEvent>`]
	type ChangedComponent: Component;
	/// The bundle used for the Trigger, ie [`Trigger<TriggerEvent,TriggerBundle>`]
	type TriggerBundle: Bundle = ();
	/// Parameters used by the handler
	type Params: 'static + Send + Sync + Default + Reflect;
	fn handle(
		commands: &mut Commands,
		entity: Entity,
		changed_component: &Self::ChangedComponent,
	);
}
