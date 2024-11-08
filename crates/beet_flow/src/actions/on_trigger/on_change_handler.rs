use super::OnChange;
use bevy::prelude::*;

/// Trait for handling an OnChange event
pub trait OnChangeHandler: 'static + Send + Sync + Sized {
	type ChangedComponent: Component;
	/// Parameters used by the handler
	type Params: 'static + Send + Sync + Default + Reflect;
	fn handle(
		commands: &mut Commands,
		action_entity: Entity,
		action_component: &OnChange<Self>,
		changed_entity: Entity,
		changed_component: &Self::ChangedComponent,
	);
}
