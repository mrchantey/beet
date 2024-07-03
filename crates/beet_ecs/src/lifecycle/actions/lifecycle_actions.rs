use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;

/// Minimal traits generally required for an action component.
pub trait GenericActionComponent:
Clone + Component + FromReflect + GetTypeRegistration
{
}
impl<T: Clone + Component + FromReflect + GetTypeRegistration>
GenericActionComponent for T
{
}
/// Minimal traits generally required for an action event.
pub trait GenericActionEvent:
	Clone + Event + FromReflect + GetTypeRegistration
{
}
impl<T: Clone + Event + FromReflect + GetTypeRegistration>
	GenericActionEvent for T
{
}
