use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;

pub trait GenericActionComponent:
	Default + Clone + Component + FromReflect + GetTypeRegistration
{
}
impl<T: Default + Clone + Component + FromReflect + GetTypeRegistration>
	GenericActionComponent for T
{
}
