use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use std::fmt::Debug;

pub trait ActionTypes {
	/// Register components via [`World::init_component`]
	fn register_components(world: &mut World);
	/// Register types via [`TypeRegistry::register`]
	fn register_types(type_registry: &mut TypeRegistry);

	/// Create an [`AppTypeRegistry`] with all types in this module registered
	fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		Self::register_types(&mut registry.write());
		registry
	}
}


pub trait BeetModule:
	'static + Send + Sync + Debug + Clone + ActionSystems + ActionTypes
{
}
impl<T> BeetModule for T where
	T: 'static + Send + Sync + Debug + Clone + ActionSystems + ActionTypes
{
}
