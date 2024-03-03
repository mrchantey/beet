use anyhow::Result;
use bevy_ecs::schedule::SystemConfigs;
use bevy_ecs::system::EntityCommands;
use bevy_ecs::world::EntityWorldMut;
use serde::Serialize;

pub trait Action: 'static {
	fn duplicate(&self) -> Box<dyn Action>;

	fn spawn(&self, entity: &mut EntityWorldMut<'_>);
	fn spawn_with_command(&self, entity: &mut EntityCommands);

	// fn pre_tick_system(&self) -> SystemConfigs;
	fn tick_system(&self) -> SystemConfigs;
	fn post_tick_system(&self) -> SystemConfigs;

	fn meta(&self) -> ActionMeta;
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


pub trait IntoAction: 'static + Clone + Send + Sync + Serialize {
	fn into_action(self) -> Box<dyn Action>;
	fn into_action_ref(&self) -> &dyn Action;
	fn into_action_mut(&mut self) -> &mut dyn Action;
}

impl<T: IntoAction> Action for T {
	fn duplicate(&self) -> Box<dyn Action> {
		self.into_action_ref().duplicate()
	}

	fn spawn(&self, entity: &mut EntityWorldMut<'_>) {
		self.into_action_ref().spawn(entity)
	}

	fn spawn_with_command(&self, entity: &mut EntityCommands) {
		self.into_action_ref().spawn_with_command(entity)
	}

	fn tick_system(&self) -> SystemConfigs {
		self.into_action_ref().tick_system()
	}

	fn post_tick_system(&self) -> SystemConfigs {
		self.into_action_ref().post_tick_system()
	}

	fn meta(&self) -> ActionMeta { self.into_action_ref().meta() }
}

// impl<T> IntoAction for T where
// 	T: 'static + Clone + Send + Sync + Serialize + Into<Box<dyn Action>>
// {
// }
