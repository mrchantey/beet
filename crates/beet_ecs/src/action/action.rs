use anyhow::Result;
use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::EntityCommands;
use bevy_ecs::world::EntityWorldMut;
use bevy_reflect::reflect_trait;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use std::fmt;


#[reflect_trait]
pub trait Action: 'static + Reflect + fmt::Debug {
	// [`Clone`] but boxable, theres probably a better way..
	fn duplicate(&self) -> Box<dyn Action>;
	// must be seperate so can be Boxed, ie no `impl WorldOrCommands`
	fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>);
	fn insert_from_commands(&self, entity: &mut EntityCommands);
}


// impl Action for Box<dyn Action> {
// 	fn duplicate(&self) -> Box<dyn Action> { self.as_ref().duplicate() }
// 	fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>) {
// 		self.as_ref().insert_from_world(entity)
// 	}
// 	fn insert_from_commands(&self, entity: &mut EntityCommands) {
// 		self.as_ref().insert_from_commands(entity)
// 	}
// }

pub trait ActionSystems: 'static {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}
pub trait ActionTypes: 'static {
	fn register(registry: &mut TypeRegistry);
}

pub type SetActionFunc = Box<dyn Fn(&mut EntityCommands) -> Result<()>>;

pub trait SetAction: Action {
	fn set(&mut self, func: SetActionFunc);
}
