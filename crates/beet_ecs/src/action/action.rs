use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::world::EntityWorldMut;
use bevy_reflect::reflect_trait;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct NoopActionTypes;
impl ActionTypes for NoopActionTypes {
	fn register(_: &mut TypeRegistry) {}
}


#[reflect_trait]
pub trait ActionChildComponents {
	fn insert_child_components(&self, entity: &mut EntityWorldMut<'_>);
	fn boxed_child_components(&self) -> Vec<Box<dyn Reflect>>;
}

pub trait ActionSystems: 'static {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}

pub struct ActionSystemMarker;
// impl<T> IntoSystemConfigs<ActionSystemMarker> for T where T:ActionSystems{
// 		fn into_configs(self) -> bevy_ecs::schedule::SystemConfigs {
// 				// self.
// 		}
// }


pub trait ActionTypes: 'static + Send + Sync + Debug + Clone {
	fn register(registry: &mut TypeRegistry);
}


pub trait ActionList: ActionSystems + ActionTypes {}
impl<T> ActionList for T where T: ActionSystems + ActionTypes {}
