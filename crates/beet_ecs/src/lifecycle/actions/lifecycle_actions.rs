use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;

pub trait GenericActionComponent:
	Clone + Component + FromReflect + GetTypeRegistration
{
}
impl<T: Clone + Component + FromReflect + GetTypeRegistration>
	GenericActionComponent for T
{
}
