use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;

/// Minimal traits generally required for an action generic type.
pub trait GenericActionType:
	'static + Send + Sync + Default + Clone + FromReflect + GetTypeRegistration
{
}
impl<
		T: 'static
			+ Send
			+ Sync
			+ Default
			+ Clone
			+ FromReflect
			+ GetTypeRegistration,
	> GenericActionType for T
{
}


/// Minimal traits generally required for an action component.
pub trait GenericActionComponent:
	Clone + FromReflect + GetTypeRegistration + Component
{
}
impl<T: Clone + FromReflect + GetTypeRegistration + Component>
	GenericActionComponent for T
{
}
/// Minimal traits generally required for an action event.
pub trait GenericActionEvent:
	Clone + FromReflect + GetTypeRegistration + Event
{
}
impl<T: Clone + FromReflect + GetTypeRegistration + Event> GenericActionEvent
	for T
{
}
pub trait ReflectEvent: Event + FromReflect + GetTypeRegistration {}
impl<T: Event + FromReflect + GetTypeRegistration> ReflectEvent for T {}
