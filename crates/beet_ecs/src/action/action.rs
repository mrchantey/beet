use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use std::fmt::Debug;

#[reflect_trait]
pub trait ActionChildComponents {
	fn insert_child_components(&self, entity: &mut EntityWorldMut<'_>);
	fn boxed_child_components(&self) -> Vec<Box<dyn Reflect>>;
}

// must be static for use in beet plugin
pub trait ActionSystems: 'static {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}

pub struct ActionSystemMarker;
// impl<T> IntoSystemConfigs<ActionSystemMarker> for T where T:ActionSystems{
// 		fn into_configs(self) -> bevy::schedule::SystemConfigs {
// 				// self.
// 		}
// }


pub trait ActionTypes {
	/// Register components via [`World::init_component`]
	fn register_components(world: &mut World);
	/// Register types via [`TypeRegistry::register`]
	fn register_types(type_registry: &mut TypeRegistry);
}


pub trait ActionList:
	'static + Send + Sync + Debug + Clone + ActionSystems + ActionTypes
{
}
impl<T> ActionList for T where
	T: 'static + Send + Sync + Debug + Clone + ActionSystems + ActionTypes
{
}
