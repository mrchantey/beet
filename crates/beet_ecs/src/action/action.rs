use anyhow::Result;
use bevy_app::App;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::EntityCommands;
use bevy_ecs::world::EntityWorldMut;

pub trait Action: 'static {
	fn duplicate(&self) -> Box<dyn Action>;

	fn spawn(&self, entity: &mut EntityWorldMut<'_>);
	fn spawn_with_command(&self, entity: &mut EntityCommands);

	fn meta(&self) -> ActionMeta;
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