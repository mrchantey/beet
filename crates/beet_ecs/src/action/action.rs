use anyhow::Result;
use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::EntityCommands;
use bevy_ecs::world::EntityWorldMut;

pub trait Action: 'static {
	fn duplicate(&self) -> Box<dyn Action>;
	fn meta(&self) -> ActionMeta;
	// must be seperate so can be Boxed, ie no `impl WorldOrCommands`
	fn insert_from_world(&self, entity: &mut EntityWorldMut<'_>);
	fn insert_from_commands(&self, entity: &mut EntityCommands);
}

pub trait ActionSystems: 'static {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}

pub type SetActionFunc = Box<dyn Fn(&mut EntityCommands) -> Result<()>>;

pub trait SetAction: Action {
	fn set(&mut self, func: SetActionFunc);
}

#[derive(Debug, Clone)]
pub struct ActionMeta {
	pub name: &'static str,
	pub id: usize,
}
